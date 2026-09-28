using System;
using System.IO;
using Recite.Unity;
using static ReciteUnityHeadless;

// Real managed/native asset, catalogue and contract cases used by the headless gate.
internal static class ReciteUnityNativeCases
{
    internal static void Run()
    {
        InspectCanonicalAssetInfo();
        RetainActiveRevisionAcrossCandidateUpdate();
        PoLocaleModeTransitions();
        PreservePluralMetadataFromNativePo();
        ManagedContractErrors();
        StaleChoiceAfterLaterPrompt();
        RestoreRejectsDifferentSchema();
    }

    private static void PreservePluralMetadataFromNativePo()
    {
        var path = Environment.GetEnvironmentVariable("RECITE_UNITY_PLURAL_ASSET");
        Assert(!string.IsNullOrEmpty(path) && File.Exists(path), "plural fixture path was not configured");
        var poText = "msgid \"\"\nmsgstr \"\"\n\"Language: fr-FR\\n\"\n" +
            "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n" +
            "msgctxt \"5fcf9a1f7b20211f4a92\"\n" +
            "msgid \"You have one letter.\"\n" +
            "msgid_plural \"You have {count} letters.\"\n" +
            "msgstr[0] \"Vous avez une lettre.\"\n" +
            "msgstr[1] \"Vous avez {count} lettres.\"\n";
        var asset = new ReciteDialogueAsset(File.ReadAllBytes(path));
        using (var service = new ReciteDialogueService())
        {
            service.SetInterpolationValues(new[] { ReciteInterpolationValue.Integer("count", 2) });
            service.SetPoCatalog(new[] { new RecitePoDocument("fr-FR", System.Text.Encoding.UTF8.GetBytes(poText)) });
            var line = FindLine(service.Start(asset, locale: "fr-FR"));
            Assert(line.Text == "Vous avez 2 lettres.", "native PO plural translation changed");
            Assert(line.Plural != null && line.Plural.SingularSourceText == "You have one letter." &&
                line.Plural.PluralSourceText == "You have {count} letters." && line.Plural.Count == 2 &&
                line.Plural.SelectedArm == 1, "plural source forms/count/selected arm were lost");
            const string context = "5fcf9a1f7b20211f4a92";
            var resolution = line.Plural.Resolution;
            Assert(resolution.MatchedLocale == "fr-FR" &&
                resolution.MatchedContext == context && resolution.MatchedKey == context &&
                resolution.MatchedArm == 1 && resolution.SourceFallbackArm == null &&
                resolution.Outcome == "translated" && resolution.Attempts.Count == 1,
                "plural matched resolution fields or attempt count changed");
            var attempt = resolution.Attempts[0];
            Assert(attempt.Locale == "fr-FR" && attempt.Context == context &&
                attempt.Key == context && attempt.SelectedArm == 1 &&
                attempt.Outcome == "matched", "plural ordered lookup attempt changed");
            service.End();
            service.SetPoCatalog(null);
            var fallback = FindLine(service.Start(asset, locale: "fr-FR"));
            Assert(fallback.Text == "You have 2 letters." && fallback.Plural != null &&
                fallback.Plural.SingularSourceText == "You have one letter." &&
                fallback.Plural.PluralSourceText == "You have {count} letters." &&
                fallback.Plural.Count == 2 && fallback.Plural.SelectedArm == 1 &&
                fallback.Plural.Resolution.MatchedLocale == null &&
                fallback.Plural.Resolution.MatchedContext == null &&
                fallback.Plural.Resolution.MatchedKey == null &&
                fallback.Plural.Resolution.MatchedArm == null &&
                fallback.Plural.Resolution.SourceFallbackArm == 1 &&
                fallback.Plural.Resolution.Outcome == "english_source_fallback" &&
                fallback.Plural.Resolution.Attempts.Count == 0,
                "plural source fallback lost its text or structured provenance");
        }
    }

    private static void PoLocaleModeTransitions()
    {
        var asset = new ReciteDialogueAsset(File.ReadAllBytes(
            Environment.GetEnvironmentVariable("RECITE_UNITY_SAMPLE_ASSET")));
        var poText = "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\n" +
            "msgctxt \"10000000000000000001\"\n" +
            "msgid \"The relay wakes.\"\n" +
            "msgstr \"Le relais s'éveille.\"\n\n" +
            "msgctxt \"10000000000000000001&formal\"\n" +
            "msgid \"The relay wakes.\"\n" +
            "msgstr \"Le relais vous parle.\"\n\n" +
            "msgctxt \"10000000000000000006\"\n" +
            "msgid \"The relay key clicks free.\"\n" +
            "msgstr \"La clé du relais se libère.\"\n";
        var poBytes = System.Text.Encoding.UTF8.GetBytes(poText);
        var po = new RecitePoDocument("fr", poBytes);
        poBytes[0] ^= 0xff;
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("has_key", _ => false);
            Assert(FindLine(service.Start(asset, locale: "fr")).Text == "The relay wakes.",
                "untranslated start did not use source text");
            service.End();
            service.SetPoCatalog(new[] { po });
            service.SetLocaleVariant("formal");
            Assert(FindLine(service.Start(asset, locale: "fr-CA")).Text == "Le relais vous parle.",
                "native PO catalogue did not use the preselected variant and locale fallback");
            service.SetLocaleVariant(null);
            ExpectStatus(() => service.SetPoCatalog(new[] {
                new RecitePoDocument("fr", new byte[] { 0xff })
            }), ReciteStatus.Localisation);
            Assert(service.HasActiveSession, "failed PO candidate interrupted active session");
            service.End();
            var renewed = service.Start(asset, locale: "fr-CA");
            Assert(FindLine(renewed).Text == "Le relais s'éveille.",
                "failed PO candidate replaced prior catalogue");
            var prompt = FindPrompt(renewed);
            var pendingEffect = FindEffect(service.SelectChoice(prompt.Choices[0].Id)).Effect;
            var snapshot = service.Snapshot();
            service.End();
            Assert(FindEffect(service.Restore(asset, snapshot)).Effect.Id == pendingEffect.Id,
                "PO catalogue restore changed pending effect identity");
            Assert(FindLine(service.AcknowledgeEffect(pendingEffect.Id)).Text ==
                "La clé du relais se libère.", "owned PO catalogue was not attached during restore");
            service.SetPoCatalog(null);
            service.End();
            Assert(FindLine(service.Start(asset, locale: "fr")).Text == "The relay wakes.",
                "cleared PO catalogue was reattached on next session");
        }
    }

    private static void RetainActiveRevisionAcrossCandidateUpdate()
    {
        var oldPath = Environment.GetEnvironmentVariable("RECITE_UNITY_REVISION_OLD");
        var newPath = Environment.GetEnvironmentVariable("RECITE_UNITY_REVISION_NEW");
        Assert(!string.IsNullOrEmpty(oldPath) && !string.IsNullOrEmpty(newPath),
            "revision fixture paths were not configured");
        var oldAsset = new ReciteDialogueAsset(File.ReadAllBytes(oldPath));
        var newAsset = new ReciteDialogueAsset(File.ReadAllBytes(newPath));
        var oldInfo = ReciteAssetInfo.Inspect(oldAsset);
        var newInfo = ReciteAssetInfo.Inspect(newAsset);
        Assert(oldInfo.AssetId == newInfo.AssetId, "revision fixture asset IDs differ");
        Assert(!ByteArraysEqual(oldInfo.ContentFingerprint.Digest, newInfo.ContentFingerprint.Digest),
            "same-name changed content did not change native fingerprint");
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("has_key", _ => false);
            service.Start(oldAsset);
            Assert(ByteArraysEqual(service.ActiveAssetInfo.ContentFingerprint.Digest,
                oldInfo.ContentFingerprint.Digest), "active revision does not match initial asset");
            ExpectStatus(() => service.Start(newAsset), ReciteStatus.SessionAlreadyActive);
            Assert(ByteArraysEqual(service.ActiveAssetInfo.ContentFingerprint.Digest,
                oldInfo.ContentFingerprint.Digest), "failed second start replaced active revision");
            var snapshot = service.Snapshot();
            service.End();
            Assert(service.ActiveAssetInfo == null, "End retained active asset metadata");
            ExpectStatus(() => service.Restore(newAsset, snapshot), ReciteStatus.SaveLoadIncompatibility);
            Assert(service.ActiveAssetInfo == null, "mismatched restore retained active asset metadata");
            service.Start(newAsset);
            Assert(ByteArraysEqual(service.ActiveAssetInfo.ContentFingerprint.Digest,
                newInfo.ContentFingerprint.Digest), "next session did not take new revision");
        }
    }

    private static bool ByteArraysEqual(byte[] left, byte[] right)
    {
        if (left.Length != right.Length) return false;
        for (var i = 0; i < left.Length; i++) if (left[i] != right[i]) return false;
        return true;
    }

    private static void InspectCanonicalAssetInfo()
    {
        var bytes = File.ReadAllBytes(Environment.GetEnvironmentVariable("RECITE_UNITY_SAMPLE_ASSET"));
        var info = ReciteAssetInfo.Inspect(new ReciteDialogueAsset(bytes));
        Assert(!string.IsNullOrEmpty(info.AssetId), "native asset info has no asset ID");
        Assert(!string.IsNullOrEmpty(info.SourceMapId), "native asset info has no source map ID");
        Assert(info.ContentFingerprint.Digest.Length > 0, "native content fingerprint is empty");
        var mutable = info.ContentFingerprint.Digest;
        mutable[0] ^= 0xff;
        Assert(mutable[0] != info.ContentFingerprint.Digest[0], "native fingerprint digest leaked mutable state");
        var invalid = (byte[])bytes.Clone();
        invalid[0] ^= 0xff;
        ExpectStatus(() => ReciteAssetInfo.Inspect(new ReciteDialogueAsset(invalid)), ReciteStatus.AssetLoadOrDecode);
    }

    private static void ManagedContractErrors()
    {
        var asset = new ReciteDialogueAsset(File.ReadAllBytes(
            Environment.GetEnvironmentVariable("RECITE_UNITY_SAMPLE_ASSET")));
        using (var service = new ReciteDialogueService())
        {
            ExpectStatus(() => service.SelectChoice("unknown"), ReciteStatus.NoActiveSession);
            ExpectStatus(() => service.Start(asset, startBlock: "missing_block"), ReciteStatus.UnknownStartBlock);
            Assert(!service.HasActiveSession, "failed start retained native session");
            ExpectStatus(() => service.Start(asset), ReciteStatus.MissingConditionHandler);
            Assert(!service.HasActiveSession, "missing condition retained native session");
        }
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("has_key", _ => throw new InvalidOperationException("unavailable"));
            ExpectStatus(() => service.Start(asset), ReciteStatus.ConditionEvaluation);
            Assert(!service.HasActiveSession, "condition failure retained native session");
        }
        using (var service = new ReciteDialogueService())
        {
            service.RegisterConditionValue("has_key", _ => ReciteConditionValue.Enum("ready"));
            ExpectStatus(() => service.Start(asset), ReciteStatus.InvalidConditionResult);
            Assert(!service.HasActiveSession, "invalid condition result retained native session");
        }
        using (var service = new ReciteDialogueService())
        {
            service.RegisterConditionValue("has_key", _ => null);
            ExpectStatus(() => service.Start(asset), ReciteStatus.InvalidConditionResult);
            Assert(!service.HasActiveSession, "null condition result retained native session");
        }
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("has_key", _ => throw new InvalidOperationException("bad\0message"));
            ExpectStatus(() => service.Start(asset), ReciteStatus.ConditionEvaluation);
            Assert(!service.HasActiveSession, "malformed callback error retained native session");
        }
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("has_key", _ => false);
            var start = service.Start(asset);
            ExpectStatus(() => service.SelectChoice("unknown"), ReciteStatus.InvalidChoice);
            var prompt = FindPrompt(start);
            var chosen = service.SelectChoice(prompt.Choices[0].Id);
            var pending = FindEffect(chosen).Effect;
            ExpectStatus(() => service.AcknowledgeEffect("wrong"), ReciteStatus.EffectAcknowledgement);
            service.AcknowledgeEffect(pending.Id);
            ExpectStatus(() => service.SelectChoice(prompt.Choices[0].Id), ReciteStatus.StaleChoice);
            service.End();
            ExpectStatus(() => service.Snapshot(), ReciteStatus.NoActiveSession);
            ExpectStatus(() => service.Restore(asset, new ReciteSessionSnapshot(new byte[] { 0x00 })),
                ReciteStatus.SaveLoadIncompatibility);
            Assert(!service.HasActiveSession, "failed restore retained native session");
        }
    }

    private static void StaleChoiceAfterLaterPrompt()
    {
        var path = Environment.GetEnvironmentVariable("RECITE_UNITY_CONFORMANCE_ASSET");
        var asset = new ReciteDialogueAsset(File.ReadAllBytes(path));
        using (var service = new ReciteDialogueService())
        {
            service.RegisterCondition("trusts", _ => true);
            var first = FindPrompt(service.Start(asset));
            Assert(first != null && first.Choices.Count == 2, "conformance first prompt changed");
            var oldChoice = first.Choices[0].Id;
            var effect = FindEffect(service.SelectChoice(oldChoice)).Effect;
            var later = FindPrompt(service.AcknowledgeEffect(effect.Id));
            Assert(later != null && later.Choices.Count == 1, "conformance later prompt changed");
            ExpectStatus(() => service.SelectChoice(oldChoice), ReciteStatus.StaleChoice);
            ExpectStatus(() => service.SelectChoice("unknown"), ReciteStatus.InvalidChoice);
        }
    }

    private static void RestoreRejectsDifferentSchema()
    {
        var first = new ReciteDialogueAsset(File.ReadAllBytes(
            Environment.GetEnvironmentVariable("RECITE_UNITY_SCHEMA_A")));
        var second = new ReciteDialogueAsset(File.ReadAllBytes(
            Environment.GetEnvironmentVariable("RECITE_UNITY_SCHEMA_B")));
        var firstInfo = ReciteAssetInfo.Inspect(first);
        var secondInfo = ReciteAssetInfo.Inspect(second);
        Assert(firstInfo.AssetId == secondInfo.AssetId, "schema fixture changed asset ID");
        Assert(firstInfo.SchemaFingerprint != null && secondInfo.SchemaFingerprint != null &&
            !ByteArraysEqual(firstInfo.SchemaFingerprint.Digest, secondInfo.SchemaFingerprint.Digest),
            "schema fixture has no distinct canonical fingerprints");
        using (var service = new ReciteDialogueService())
        {
            Assert(FindPrompt(service.Start(first)) != null, "schema fixture did not reach its prompt");
            var snapshot = service.Snapshot();
            service.End();
            ExpectStatus(() => service.Restore(second, snapshot), ReciteStatus.SchemaMismatch);
            Assert(!service.HasActiveSession && service.ActiveAssetInfo == null,
                "schema-mismatched restore retained native session or metadata");
            Assert(service.Restore(first, snapshot).Events.Count == 0,
                "matching-schema restore failed after rejected candidate");
        }
    }

}
