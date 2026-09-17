# Writer packaging and desktop integration

Cross-platform writer installation and packaging are not implemented or accepted
yet. Running the workspace binary and its headless tests does not establish an
installed application on Linux, macOS or Windows. Release smoke evidence is
tracked in [#79](https://github.com/plethu/recite/issues/79); platform acceptance
also remains part of the GUI milestone.

## Required deep-link handling

For each supported desktop platform, the installed package must:

- Register `recite://` with the desktop: a Linux desktop entry and scheme
  association, macOS bundle URL types and URL-open event handling, or Windows
  protocol registration and activation handling.
- Open the project identified by a link before resolving its scene, passage,
  catalogue or translation-queue location. Report missing/moved projects and
  unavailable targets without silently selecting a different project.
- Deliver links to an already-running writer and focus the appropriate window.
  Running-window delivery must pass through the same route validation and draft
  guards as Back/Forward. Neither startup nor activation may discard source or
  translation edits or create competing writers for the same files.
- Preserve encoded paths, Unicode, spaces and queue queries when handing off the
  URL. Parse it as data; never interpolate it into a shell command. Reject
  malformed or unsupported routes without modifying project content.
- Keep registration usable after an upgrade and remove registrations owned by
  the package on uninstall, without removing another application's association.

The installation smoke matrix must exercise cold launch and running-window
activation from an actual desktop link, correct project/scene/catalogue
selection, queue search/filter/page restoration, unsaved-edit refusal, malformed
links, upgrade and uninstall. Record results and limits separately for each OS.

The [route format, Copy link control and `--route` startup argument](navigation.md)
exist now. They are inputs to this work, not evidence that desktop registration,
project activation or running-window delivery is complete.
