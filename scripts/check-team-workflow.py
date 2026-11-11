#!/usr/bin/env python3
"""Exercise a multi-author repository journey on the team project with separate checkouts.

Two writers, a localiser and a programmer each work in their own clone of a shared repository.
Their changes merge through ordinary Git, and a clean CI checkout validates, compiles and traces
the result with the `recite` CLI. Integration problems that Git merges silently must stay visible
and block the build until a person repairs them.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FIXTURE = REPO / "fixtures/realistic/team-pack"
SCHEMA_SOURCE = "schema/team.schema.toml"
SCHEMA = "schema/generated/team.schema.json"
ASSET = "build/crossing.recitec"
CODE = re.compile(r"(?:error|warning)\[(RECITE_[A-Z]+[0-9]+)\]")


def git_environment(home):
    environment = {
        key: value for key, value in os.environ.items() if not key.startswith(("GIT_", "RECITE_"))
    }
    settings = {
        "user.name": "Team Member",
        "user.email": "team@example.invalid",
        "init.defaultBranch": "main",
        "commit.gpgsign": "false",
        "pull.rebase": "false",
    }
    environment.update(
        HOME=str(home),
        GIT_CONFIG_NOSYSTEM="1",
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_CONFIG_COUNT=str(len(settings)),
    )
    for index, (key, value) in enumerate(settings.items()):
        environment[f"GIT_CONFIG_KEY_{index}"] = key
        environment[f"GIT_CONFIG_VALUE_{index}"] = value
    return environment


class Team:
    def __init__(self, recite, scratch):
        self.recite_path = recite
        self.scratch = scratch
        self.environment = git_environment(scratch / "home")
        self.origin = scratch / "origin.git"

    def run(self, cwd, *args, success=True):
        result = subprocess.run(
            args, cwd=cwd, env=self.environment, text=True, capture_output=True, timeout=60
        )
        if (result.returncode == 0) != success:
            raise RuntimeError(
                f"{' '.join(map(str, args))} in {cwd}: exit {result.returncode}\n"
                f"{result.stdout}\n{result.stderr}"
            )
        return result.stdout + result.stderr

    def git(self, cwd, *args, success=True):
        return self.run(cwd, "git", *args, success=success)

    def recite(self, cwd, *args, success=True):
        return self.run(cwd, str(self.recite_path), *args, success=success)

    def clone(self, name):
        checkout = self.scratch / name
        self.git(self.scratch, "clone", "--quiet", str(self.origin), name)
        return checkout

    def share(self, checkout, message):
        """Commit everything, merge the shared main branch and publish the result."""
        self.git(checkout, "add", "--all")
        self.git(checkout, "commit", "--quiet", "--message", message)
        self.git(checkout, "pull", "--quiet", "--no-edit", "origin", "main")
        self.git(checkout, "push", "--quiet", "origin", "HEAD:main")

    def update(self, checkout):
        self.git(checkout, "pull", "--quiet", "--no-edit", "origin", "main")


def edit(checkout, path, old, new):
    file = checkout / path
    text = file.read_text()
    if text.count(old) != 1:
        raise RuntimeError(f"{path}: expected exactly one {old!r}")
    file.write_text(text.replace(old, new))


def structured_codes(output):
    codes = []
    for line in output.splitlines():
        record = json.loads(line)
        if record.get("event") == "command.result":
            codes += [item["code"] for item in record.get("data", {}).get("diagnostics", [])]
    return codes


def build(team, checkout):
    """The documented CI gate for a project checkout; it returns the compiled bytes."""
    (checkout / "build").mkdir(exist_ok=True)
    team.recite(checkout, "validate", "scenes", "--schema", SCHEMA)
    team.recite(checkout, "id", "check", "scenes", "--since", "v1")
    team.recite(checkout, "compile", "scenes", "--schema", SCHEMA, "-o", ASSET)
    team.recite(checkout, "project", "validate", ".")
    return (checkout / ASSET).read_bytes()


def trace(team, checkout, fixture):
    output = team.recite(checkout, "trace", ASSET, "--block", "dock", "--fixture", fixture)
    return json.loads(output)


def line_text(document, line_id):
    for event in document["events"]:
        if event["type"] == "line":
            line = event["line"]
        elif event["type"] == "prompt":
            line = event["prompt"]["line"]
        else:
            continue
        if line and line["id"] == line_id:
            return line["text"]
    raise RuntimeError(f"trace has no line {line_id}")


def refused(team, checkout, expected_code):
    """Integration problems fail validation and compilation and leave the last asset intact."""
    asset = checkout / ASSET
    before = asset.read_bytes() if asset.exists() else None
    for command in ("validate", "compile"):
        args = [command, "scenes", "--schema", SCHEMA, "--output-format", "structured"]
        if command == "compile":
            args += ["-o", ASSET]
        codes = structured_codes(team.recite(checkout, *args, success=False))
        if expected_code not in codes:
            raise RuntimeError(f"{command} reported {codes}, expected {expected_code}")
    after = asset.read_bytes() if asset.exists() else None
    if after != before:
        raise RuntimeError("a refused build replaced the last compiled asset")


def seed(team):
    team.git(team.scratch, "init", "--quiet", "--bare", str(team.origin))
    seed = team.clone("seed")
    shutil.copytree(FIXTURE, seed, dirs_exist_ok=True, ignore=shutil.ignore_patterns("build"))
    (seed / ".gitignore").write_text("build/\n")
    team.git(seed, "add", "--all")
    team.git(seed, "commit", "--quiet", "--message", "Start the crossing project")
    # The first published version freezes its stable IDs.
    team.git(seed, "tag", "v1")
    team.git(seed, "push", "--quiet", "origin", "main", "v1")


def independent_contributions(team, writer_a, writer_b, localiser):
    """Two writers change different scenes and a localiser changes a translation."""
    edit(
        writer_a,
        "scenes/deck.recite",
        "  Keep the lantern low. The current pulls hardest near the old pilings.\n",
        "  Keep the lantern low. The current pulls hardest near the old pilings.\n"
        "> @deck_gulls\n"
        "  Gulls settle on the rail and refuse to move for anyone.\n",
    )
    team.share(writer_a, "Add the gulls to the deck")
    edit(
        writer_b,
        "scenes/office.recite",
        "I see a name. Whether it is yours is another question.",
        "I see a name. Whether it is yours is a separate question.",
    )
    team.share(writer_b, "Sharpen the wary clerk")
    edit(
        localiser,
        "locales/fr.po",
        'msgstr "Encore du brouillard sur le fleuve. Le bac part quand la cloche se tait."',
        'msgstr "Encore la brume sur le fleuve. Le bac part quand la cloche se tait."',
    )
    edit(
        localiser,
        "locales/fr.po",
        '#. speaker: pilot\n#: scenes/dock.recite:4:3\nmsgctxt "dock_greeting"',
        "#. speaker: pilot\n# Translator: brume reads as softer river fog.\n"
        '#: scenes/dock.recite:4:3\nmsgctxt "dock_greeting"',
    )
    team.share(localiser, "Revise the French greeting")

    ci = team.clone("ci-integrated")
    build(team, ci)
    team.recite(ci, "project", "check-fresh", ".")
    status = json.loads(team.recite(ci, "catalog", "status", "--format", "json"))
    attention = {item["context"]: item["state"] for item in status["catalogs"][0]["attention"]}
    assert attention == {"office_wary": "source_changed", "deck_gulls": "missing"}, attention
    source = trace(team, ci, "fixture-source.toml")
    assert line_text(source, "deck_gulls").startswith("Gulls settle"), "writer A's line is lost"
    french = trace(team, ci, "fixture-fr.toml")
    reworded = "I see a name. Whether it is yours is a separate question."
    assert line_text(french, "office_wary") == reworded, "writer B's wording is lost"
    assert line_text(french, "dock_greeting").startswith("Encore la brume"), "translation lost"
    po = (ci / "locales/fr.po").read_text()
    assert "# Translator: brume reads as softer river fog." in po, "translator comment lost"
    for author in (writer_a, writer_b, localiser):
        team.update(author)


def same_line_conflict(team, writer_a, writer_b):
    """Both writers reword the greeting; Git stops and Recite refuses committed markers."""
    greeting = "Fog on the river again."
    edit(writer_a, "scenes/dock.recite", greeting, "Fog on the river, thick again.")
    team.share(writer_a, "Thicken the fog")
    edit(writer_b, "scenes/dock.recite", greeting, "River fog again.")
    team.git(writer_b, "commit", "--quiet", "--all", "--message", "Shorten the greeting")
    team.git(writer_b, "pull", "--quiet", "--no-edit", "origin", "main", success=False)
    text = (writer_b / "scenes/dock.recite").read_text()
    assert "<<<<<<<" in text, "Git did not record the same-line conflict"
    refused(team, writer_b, "RECITE_PARSE001")
    resolved = re.sub(
        r"<<<<<<< [^\n]*\n.*?=======\n(.*?)>>>>>>> [^\n]*\n",
        lambda match: match.group(1),
        text,
        flags=re.DOTALL,
    )
    (writer_b / "scenes/dock.recite").write_text(resolved)
    team.git(writer_b, "add", "scenes/dock.recite")
    team.git(writer_b, "commit", "--quiet", "--no-edit")
    build(team, writer_b)
    team.git(writer_b, "push", "--quiet", "origin", "HEAD:main")
    team.update(writer_a)


def duplicate_frozen_id(team, writer_a, writer_b):
    """Both writers pick the same new ID in different files; Git merges, Recite refuses."""
    edit(
        writer_a,
        "scenes/deck.recite",
        "-> scenes/office.recite::arrival\n",
        "> @harbour_note\n  A rope creaks somewhere below the deck.\n"
        "-> scenes/office.recite::arrival\n",
    )
    team.share(writer_a, "Add a deck note")
    edit(
        writer_b,
        "scenes/office.recite",
        ":: arrival speaker=narrator\n",
        ":: arrival speaker=narrator\n> @harbour_note\n  The office lamp is still burning.\n",
    )
    team.share(writer_b, "Add an office note")
    refused(team, writer_b, "RECITE_ID003")
    # The writer who did not publish first keeps the old ID on the published line and gives
    # their own line a fresh one.
    lines = (writer_b / "scenes/office.recite").read_text().splitlines()
    header = next(number for number, line in enumerate(lines, 1) if "@harbour_note" in line)
    team.recite(writer_b, "id", "reassign", f"scenes/office.recite:{header}", "scenes")
    build(team, writer_b)
    team.share(writer_b, "Give the office note its own ID")
    team.update(writer_a)


def broken_reference(team, writer_a, writer_b):
    """A rename merges cleanly with a new link to the old name; Recite reports the link."""
    for path in ("scenes/office.recite", "scenes/deck.recite"):
        text = (writer_b / path).read_text()
        text = text.replace(":: arrival speaker", ":: far_bank speaker")
        text = text.replace("-> arrival\n", "-> far_bank\n")
        text = text.replace("office.recite::arrival", "office.recite::far_bank")
        (writer_b / path).write_text(text)
    build(team, writer_b)
    team.share(writer_b, "Rename the arrival block")
    edit(
        writer_a,
        "scenes/dock.recite",
        "    -> scenes/office.recite::office\n",
        "    -> scenes/office.recite::office\n"
        "  ? @dock_wait\n    Wait for the fog to lift.\n    -> scenes/office.recite::arrival\n",
    )
    team.git(writer_a, "commit", "--quiet", "--all", "--message", "Let the player wait")
    team.git(writer_a, "pull", "--quiet", "--no-edit", "origin", "main")
    refused(team, writer_a, "RECITE_VALIDATE007")
    stale_link, renamed = "-> scenes/office.recite::arrival", "-> scenes/office.recite::far_bank"
    edit(writer_a, "scenes/dock.recite", stale_link, renamed)
    build(team, writer_a)
    team.share(writer_a, "Follow the renamed arrival block")
    team.update(writer_b)


def removed_frozen_id(team, writer_b):
    """Deleting a published line is reported against the frozen release and needs consent."""
    text = (writer_b / "scenes/office.recite").read_text()
    removed = re.sub(r"  :case wary\n(    .*\n)+", "", text)
    (writer_b / "scenes/office.recite").write_text(removed)
    team.recite(writer_b, "validate", "scenes", "--schema", SCHEMA)
    output = team.recite(writer_b, "id", "check", "scenes", "--since", "v1", success=False)
    assert "office_wary" in output, output
    consent = ["--since", "v1", "--allow-removed", "office_wary"]
    team.recite(writer_b, "id", "check", "scenes", *consent)
    team.git(writer_b, "checkout", "--quiet", "--", "scenes/office.recite")


def schema_change(team, programmer, writer_b):
    """The programmer renames a condition; stale output, explicit export and repair."""
    team.update(programmer)
    build(team, programmer)
    published = (programmer / ASSET).read_bytes()
    edit(programmer, SCHEMA_SOURCE, "[conditions.trust_of]", "[conditions.clerk_trust]")
    fresh = programmer / "build/fresh.schema.json"
    export = ["schema", "export", "--schema", SCHEMA_SOURCE, "--producer-kind", "standalone"]
    export += ["--producer-id", "team-pack", "--output"]
    team.recite(programmer, *export, str(fresh))
    compare = ["schema", "check-fresh", "--expected", SCHEMA, "--actual", str(fresh)]
    stale = team.recite(programmer, *compare, success=False)
    assert json.loads(stale)["status"] == "mismatch", stale
    # Regeneration is an explicit step; it changes the generated file reviewers see.
    team.recite(programmer, *export, SCHEMA)
    assert "clerk_trust" in (programmer / SCHEMA).read_text()
    refused(team, programmer, "RECITE_VALIDATE034")
    assert (programmer / ASSET).read_bytes() == published, "invalid schema replaced the asset"
    codes = CODE.findall(team.recite(programmer, "project", "check-fresh", ".", success=False))
    assert "RECITE_FRESH002" in codes, codes
    edit(programmer, "scenes/office.recite", ":match trust_of(clerk)", ":match clerk_trust(clerk)")
    build(team, programmer)
    # Host fixtures answer conditions by name; an old answer does not stand in for the new query.
    trace_args = ["trace", ASSET, "--block", "dock", "--fixture", "fixture-fr.toml"]
    missing = team.recite(programmer, *trace_args, success=False)
    assert "clerk_trust(clerk)" in missing, missing
    for fixture in ("fixture-source.toml", "fixture-fr.toml"):
        edit(programmer, fixture, '"trust_of(clerk)"', '"clerk_trust(clerk)"')
        trace(team, programmer, fixture)
    team.share(programmer, "Rename the clerk trust condition")
    team.update(writer_b)


def reproducible(team):
    """Two clean checkouts build identical content and traces in both locales."""
    first, second = team.clone("ci-first"), team.clone("ci-second")
    assert build(team, first) == build(team, second), "clean checkouts compiled differently"
    for fixture in ("fixture-source.toml", "fixture-fr.toml"):
        assert trace(team, first, fixture) == trace(team, second, fixture), fixture
    return first


def structured_failures(team, checkout):
    """Stale assets, unanswered fixture questions and invalid schema fail with stable codes."""
    edit(checkout, "scenes/deck.recite", "Gulls settle", "Two gulls settle")
    codes = CODE.findall(team.recite(checkout, "project", "check-fresh", ".", success=False))
    assert "RECITE_FRESH001" in codes, codes
    team.git(checkout, "checkout", "--quiet", "--", "scenes/deck.recite")
    unanswered = checkout / "build/unanswered.toml"
    unanswered.write_text('[conditions]\n"has_cargo(lantern)" = true\n')
    unanswered_trace = ["trace", ASSET, "--block", "dock", "--fixture", str(unanswered)]
    result = team.recite(
        checkout, *unanswered_trace, "--output-format", "structured", success=False
    )
    statuses = [json.loads(line).get("status") for line in result.splitlines() if line]
    assert any(status and status != "succeeded" for status in statuses), result
    (checkout / SCHEMA).write_text('{"schema_version": 1, "conditions": {"has_cargo": 3}}\n')
    validate = ["validate", "scenes", "--schema", SCHEMA, "--output-format", "structured"]
    codes = structured_codes(team.recite(checkout, *validate, success=False))
    assert codes, "an invalid schema must produce structured diagnostics"


def check(recite, scratch):
    team = Team(recite, scratch)
    seed(team)
    writer_a, writer_b = team.clone("writer-a"), team.clone("writer-b")
    localiser, programmer = team.clone("localiser"), team.clone("programmer")
    independent_contributions(team, writer_a, writer_b, localiser)
    same_line_conflict(team, writer_a, writer_b)
    duplicate_frozen_id(team, writer_a, writer_b)
    broken_reference(team, writer_a, writer_b)
    removed_frozen_id(team, writer_b)
    schema_change(team, programmer, writer_b)
    structured_failures(team, reproducible(team))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recite", type=Path, required=True)
    args = parser.parse_args()
    recite = args.recite.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="recite-team-") as scratch:
        check(recite, Path(scratch))
    print(
        "Separate-checkout integration, conflict and identity refusal, schema refresh and "
        "reproducible builds passed."
    )


if __name__ == "__main__":
    main()
