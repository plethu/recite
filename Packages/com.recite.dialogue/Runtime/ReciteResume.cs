using System;
using System.Collections.Generic;

namespace Recite.Unity
{
    /// <summary>
    /// How far a resumed save may move from the asset it was saved against, in
    /// increasing order of change. Values match the C ABI's
    /// <c>RECITE_RESUME_LEVEL_*</c> constants.
    /// </summary>
    public enum ReciteResumeLevel : uint
    {
        /// <summary>Only the exact asset the session was saved against.</summary>
        Exact = 0,
        /// <summary>The asset changed, but not the block the session was saved in.</summary>
        UnchangedBlock = 1,
        /// <summary>Only text, speakers, interpolation or metadata changed in the saved block.</summary>
        TextEdited = 2,
        /// <summary>The saved position was found again through its stable IDs.</summary>
        Relocated = 3,
    }

    /// <summary>What changed between the saved asset and the one a save resumed in.</summary>
    public sealed class ReciteResumeReport
    {
        public ReciteResumeReport(string level, string blockId, string prompt, bool schemaChanged, bool compilerChanged)
        {
            Level = level ?? string.Empty;
            BlockId = blockId ?? string.Empty;
            Prompt = prompt ?? string.Empty;
            SchemaChanged = schemaChanged;
            CompilerChanged = compilerChanged;
        }

        /// <summary><c>exact</c>, <c>unchanged_block</c>, <c>text_edited</c> or <c>relocated</c>.</summary>
        public string Level { get; }

        public string BlockId { get; }

        /// <summary><c>none</c>, <c>kept</c> or <c>reevaluated</c>.</summary>
        public string Prompt { get; }

        public bool SchemaChanged { get; }

        public bool CompilerChanged { get; }
    }

    /// <summary>The line or prompt that located a saved point.</summary>
    public sealed class ReciteResumeLandmark
    {
        public ReciteResumeLandmark(string kind, string lineId, IReadOnlyList<string> choiceIds)
        {
            Kind = kind ?? string.Empty;
            LineId = lineId;
            ChoiceIds = choiceIds ?? Array.Empty<string>();
        }

        /// <summary><c>line</c> or <c>prompt</c>.</summary>
        public string Kind { get; }

        public string LineId { get; }

        public IReadOnlyList<string> ChoiceIds { get; }
    }

    /// <summary>
    /// A save that could not resume in the provided asset. No session is active.
    /// Deferred effects and a pending blocking effect are returned exactly as saved,
    /// so the game can honour them before it restarts or ends the conversation.
    /// </summary>
    public sealed class ReciteResumeFallback
    {
        public ReciteResumeFallback(
            string reason,
            string requiredLevel,
            string allowedLevel,
            ReciteResumeLandmark reasonLandmark,
            string blockId,
            bool blockAvailable,
            ReciteResumeLandmark landmark,
            bool ended,
            IReadOnlyList<ReciteEffect> deferredEffects,
            ReciteEffect pendingEffect,
            IReadOnlyList<string> selectedChoiceHistory,
            string locale)
        {
            Reason = reason ?? string.Empty;
            RequiredLevel = requiredLevel;
            AllowedLevel = allowedLevel;
            ReasonLandmark = reasonLandmark;
            BlockId = blockId ?? string.Empty;
            BlockAvailable = blockAvailable;
            Landmark = landmark;
            Ended = ended;
            DeferredEffects = deferredEffects ?? Array.Empty<ReciteEffect>();
            PendingEffect = pendingEffect;
            SelectedChoiceHistory = selectedChoiceHistory ?? Array.Empty<string>();
            Locale = locale;
        }

        /// <summary>
        /// <c>block_removed</c>, <c>landmark_removed</c>, <c>landmark_moved</c>,
        /// <c>landmark_ambiguous</c>, <c>statements_changed</c>, <c>branch_removed</c>
        /// or <c>level_refused</c>.
        /// </summary>
        public string Reason { get; }

        /// <summary>For <c>level_refused</c>: the level the save needed.</summary>
        public string RequiredLevel { get; }

        /// <summary>For <c>level_refused</c>: the most the host allowed.</summary>
        public string AllowedLevel { get; }

        /// <summary>The line or prompt the reason refers to, if any.</summary>
        public ReciteResumeLandmark ReasonLandmark { get; }

        /// <summary>The block the session was saved in.</summary>
        public string BlockId { get; }

        /// <summary>Whether the provided asset still contains that block, so it can be restarted.</summary>
        public bool BlockAvailable { get; }

        /// <summary>The last line or prompt before the saved point, if any.</summary>
        public ReciteResumeLandmark Landmark { get; }

        /// <summary>
        /// Whether the conversation had already ended when it was saved. Its deferred effects were
        /// delivered with the end, so there is nothing to apply, restart or reconcile.
        /// </summary>
        public bool Ended { get; }

        public IReadOnlyList<ReciteEffect> DeferredEffects { get; }

        /// <summary>The blocking effect that was waiting, with its original request ID, or null.</summary>
        public ReciteEffect PendingEffect { get; }

        public IReadOnlyList<string> SelectedChoiceHistory { get; }

        public string Locale { get; }
    }

    /// <summary>
    /// The result of <see cref="ReciteDialogueService.Resume"/>: either a resumed
    /// session with its first batch, or a fallback with no active session.
    /// </summary>
    public sealed class ReciteRestoreResult
    {
        private ReciteRestoreResult(ReciteResumeReport report, ReciteOutputBatch batch, ReciteResumeFallback fallback)
        {
            Report = report;
            Batch = batch;
            Fallback = fallback;
        }

        internal static ReciteRestoreResult Resumed(ReciteResumeReport report, ReciteOutputBatch batch)
        {
            return new ReciteRestoreResult(
                report ?? throw new ArgumentNullException(nameof(report)),
                batch ?? throw new ArgumentNullException(nameof(batch)),
                null);
        }

        internal static ReciteRestoreResult FellBack(ReciteResumeFallback fallback)
        {
            return new ReciteRestoreResult(null, null, fallback ?? throw new ArgumentNullException(nameof(fallback)));
        }

        public bool IsResumed => Report != null;

        /// <summary>Set when the save resumed.</summary>
        public ReciteResumeReport Report { get; }

        /// <summary>
        /// The first batch of a resumed save: a re-presented prompt, a re-emitted
        /// blocking effect, or the dialogue that follows. Null for a fallback.
        /// </summary>
        public ReciteOutputBatch Batch { get; }

        /// <summary>Set when the save could not resume.</summary>
        public ReciteResumeFallback Fallback { get; }
    }
}
