using System;
using System.Collections.Generic;

namespace Recite.Unity
{
    public sealed class ReciteOutputBatch
    {
        public ReciteOutputBatch(ushort batchFormatVersion, IReadOnlyList<ReciteOutput> events)
        {
            BatchFormatVersion = batchFormatVersion;
            Events = events ?? Array.Empty<ReciteOutput>();
        }

        public ushort BatchFormatVersion { get; }

        public IReadOnlyList<ReciteOutput> Events { get; }
    }

    public abstract class ReciteOutput
    {
        protected ReciteOutput(string kind)
        {
            Kind = kind;
        }

        public string Kind { get; }
    }

    public sealed class ReciteLineOutput : ReciteOutput
    {
        public ReciteLineOutput(ReciteLine line)
            : base("line")
        {
            Line = line;
        }

        public ReciteLine Line { get; }
    }

    public sealed class RecitePromptOutput : ReciteOutput
    {
        public RecitePromptOutput(ReciteLine line, IReadOnlyList<ReciteChoice> choices)
            : base("prompt")
        {
            Line = line;
            Choices = choices ?? Array.Empty<ReciteChoice>();
        }

        public ReciteLine Line { get; }

        public IReadOnlyList<ReciteChoice> Choices { get; }
    }

    public sealed class ReciteEffectOutput : ReciteOutput
    {
        public ReciteEffectOutput(ReciteEffect effect)
            : base("effect")
        {
            Effect = effect;
        }

        public ReciteEffect Effect { get; }
    }

    public sealed class ReciteEndOutput : ReciteOutput
    {
        public ReciteEndOutput(IReadOnlyList<ReciteEffect> deferredEffects)
            : base("end")
        {
            DeferredEffects = deferredEffects ?? Array.Empty<ReciteEffect>();
        }

        public IReadOnlyList<ReciteEffect> DeferredEffects { get; }
    }

    public sealed class ReciteLine
    {
        public ReciteLine(string id, string sourceText, string text, string speaker, IReadOnlyList<ReciteMetadata> metadata)
        {
            Id = id ?? string.Empty;
            SourceText = sourceText ?? string.Empty;
            Text = text ?? string.Empty;
            Speaker = speaker;
            Metadata = metadata ?? Array.Empty<ReciteMetadata>();
        }

        public string Id { get; }

        public string SourceText { get; }

        public string Text { get; }

        public string Speaker { get; }

        public IReadOnlyList<ReciteMetadata> Metadata { get; }
    }

    public sealed class ReciteChoice
    {
        public ReciteChoice(string id, string sourceText, string text, IReadOnlyList<ReciteMetadata> metadata, ReciteChoiceEcho echo, ReciteChoiceAvailability availability)
        {
            Id = id ?? string.Empty;
            SourceText = sourceText ?? string.Empty;
            Text = text ?? string.Empty;
            Metadata = metadata ?? Array.Empty<ReciteMetadata>();
            Echo = echo ?? ReciteChoiceEcho.None;
            Availability = availability ?? ReciteChoiceAvailability.Available;
        }

        public string Id { get; }

        public string SourceText { get; }

        public string Text { get; }

        public IReadOnlyList<ReciteMetadata> Metadata { get; }

        public ReciteChoiceEcho Echo { get; }

        public ReciteChoiceAvailability Availability { get; }
    }

    public sealed class ReciteChoiceEcho
    {
        public static readonly ReciteChoiceEcho None = new ReciteChoiceEcho("none", null);

        public ReciteChoiceEcho(string kind, string explicitLineId)
        {
            Kind = kind ?? "none";
            ExplicitLineId = explicitLineId;
        }

        public string Kind { get; }

        public string ExplicitLineId { get; }
    }

    public sealed class ReciteChoiceAvailability
    {
        public static readonly ReciteChoiceAvailability Available = new ReciteChoiceAvailability(true, null, null);

        public ReciteChoiceAvailability(bool isAvailable, ReciteAvailabilityReason primaryReason, ReciteAvailabilityReasonTree reasonTree)
        {
            IsAvailable = isAvailable;
            PrimaryReason = primaryReason;
            ReasonTree = reasonTree;
        }

        public bool IsAvailable { get; }

        public ReciteAvailabilityReason PrimaryReason { get; }

        public ReciteAvailabilityReasonTree ReasonTree { get; }
    }

    public sealed class ReciteAvailabilityReason
    {
        public ReciteAvailabilityReason(string id, string sourceText, string text, IReadOnlyList<ReciteReasonArg> args)
        {
            Id = id ?? string.Empty;
            SourceText = sourceText ?? string.Empty;
            Text = text ?? string.Empty;
            Args = args ?? Array.Empty<ReciteReasonArg>();
        }

        public string Id { get; }

        public string SourceText { get; }

        public string Text { get; }

        public IReadOnlyList<ReciteReasonArg> Args { get; }
    }

    public sealed class ReciteReasonArg
    {
        public ReciteReasonArg(string name, ReciteTaggedValue value)
        {
            Name = name ?? string.Empty;
            Value = value ?? ReciteTaggedValue.Null;
        }

        public string Name { get; }

        public ReciteTaggedValue Value { get; }
    }

    public sealed class ReciteAvailabilityReasonTree
    {
        public ReciteAvailabilityReasonTree(string kind, IReadOnlyList<ReciteAvailabilityReasonTree> children, ReciteAvailabilityReason reason, string text)
        {
            Kind = kind ?? string.Empty;
            Children = children ?? Array.Empty<ReciteAvailabilityReasonTree>();
            Reason = reason;
            Text = text;
        }

        public string Kind { get; }

        public IReadOnlyList<ReciteAvailabilityReasonTree> Children { get; }

        public ReciteAvailabilityReason Reason { get; }

        public string Text { get; }
    }

    public sealed class ReciteEffect
    {
        public ReciteEffect(string id, string mode, string function, IReadOnlyList<ReciteTaggedValue> args, string sourceFile, uint sourceLine, uint sourceColumn)
        {
            Id = id ?? string.Empty;
            Mode = mode ?? string.Empty;
            Function = function ?? string.Empty;
            Args = args ?? Array.Empty<ReciteTaggedValue>();
            SourceFile = sourceFile ?? string.Empty;
            SourceLine = sourceLine;
            SourceColumn = sourceColumn;
        }

        public string Id { get; }

        public string Mode { get; }

        public string Function { get; }

        public IReadOnlyList<ReciteTaggedValue> Args { get; }

        public string SourceFile { get; }

        public uint SourceLine { get; }

        public uint SourceColumn { get; }
    }

    public sealed class ReciteMetadata
    {
        public ReciteMetadata(string key, ReciteTaggedValue value)
        {
            Key = key ?? string.Empty;
            Value = value ?? ReciteTaggedValue.Null;
        }

        public string Key { get; }

        public ReciteTaggedValue Value { get; }
    }

    public sealed class ReciteTaggedValue
    {
        public static readonly ReciteTaggedValue Null = new ReciteTaggedValue("null", null);

        public ReciteTaggedValue(string kind, object value)
        {
            Kind = kind ?? "null";
            Value = value;
        }

        public string Kind { get; }

        public object Value { get; }
    }
}
