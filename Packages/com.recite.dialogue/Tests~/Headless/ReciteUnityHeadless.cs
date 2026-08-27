using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Recite.Unity;
using Recite.Unity.Native;

internal static class ReciteUnityHeadless
{
    private static int Main()
    {
        try
        {
            DecodeV0Batch();
            RejectUnknownBatchVersion();
            PreserveTypedConditionArguments();
            RejectMalformedTypedConditionArguments();
            RejectMalformedNestedOutput();
            RejectUnknownTaggedScalarKind();
            RejectMalformedChoiceOutput();
            PreserveRestoredBlockingRequestId();
            RejectInvalidConditionPointers();
            RegisterTypedConditionApi();
            PreserveEnumConditionResult();
            PreserveConditionEnumStringBoundaries();
            PreserveSchemaMismatchStatus();
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error);
            return 1;
        }
    }

    private static void DecodeV0Batch()
    {
        var batch = ReciteMessagePack.DecodeOutputBatch(new byte[]
        {
            0x82, 0xb4, (byte)'b', (byte)'a', (byte)'t', (byte)'c', (byte)'h', (byte)'_',
            (byte)'f', (byte)'o', (byte)'r', (byte)'m', (byte)'a', (byte)'t', (byte)'_',
            (byte)'v', (byte)'e', (byte)'r', (byte)'s', (byte)'i', (byte)'o', (byte)'n', 0,
            0xa6, (byte)'e', (byte)'v', (byte)'e', (byte)'n', (byte)'t', (byte)'s', 0x90
        });

        Assert(batch.BatchFormatVersion == 0, "v0 batch version was not decoded");
        Assert(batch.Events.Count == 0, "empty v0 batch was not decoded");
    }

    private static void RejectUnknownBatchVersion()
    {
        var bytes = new byte[]
        {
            0x82, 0xb4, (byte)'b', (byte)'a', (byte)'t', (byte)'c', (byte)'h', (byte)'_',
            (byte)'f', (byte)'o', (byte)'r', (byte)'m', (byte)'a', (byte)'t', (byte)'_',
            (byte)'v', (byte)'e', (byte)'r', (byte)'s', (byte)'i', (byte)'o', (byte)'n', 1,
            0xa6, (byte)'e', (byte)'v', (byte)'e', (byte)'n', (byte)'t', (byte)'s', 0x90
        };

        try
        {
            ReciteDialogueService.DecodeBatchBytes(bytes);
        }
        catch (ReciteAdapterException error)
        {
            Assert(error.Status == ReciteStatus.Validation, "unknown batch version was not projected as validation");
            Assert((int)error.Status == -1, "validation status changed");
            return;
        }

        throw new InvalidOperationException("unsupported batch version was accepted");
    }

    private static void PreserveTypedConditionArguments()
    {
        var bytes = new byte[]
        {
            0x95,
            0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d', 0xaa, (byte)'i', (byte)'d', (byte)'e', (byte)'n', (byte)'t', (byte)'i', (byte)'f', (byte)'i', (byte)'e', (byte)'r', 0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xa5, (byte)'s', (byte)'w', (byte)'o', (byte)'r', (byte)'d',
            0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d', 0xa6, (byte)'s', (byte)'t', (byte)'r', (byte)'i', (byte)'n', (byte)'g', 0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xa5, (byte)'h', (byte)'a', (byte)'z', (byte)'e', (byte)'l',
            0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d', 0xa7, (byte)'i', (byte)'n', (byte)'t', (byte)'e', (byte)'g', (byte)'e', (byte)'r', 0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 3,
            0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d', 0xa5, (byte)'f', (byte)'l', (byte)'o', (byte)'a', (byte)'t', 0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xcb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0,
            0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d', 0xa7, (byte)'b', (byte)'o', (byte)'o', (byte)'l', (byte)'e', (byte)'a', (byte)'n', 0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xc3
        };
        var arguments = ReciteMessagePack.DecodeTypedConditionArgs(bytes);

        Assert(arguments.Count == 5, "all condition arguments were not decoded");
        Assert(arguments[0].Kind == ReciteConditionArgumentKind.Identifier && arguments[0].IdentifierValue == "sword", "identifier kind was not preserved");
        Assert(arguments[1].Kind == ReciteConditionArgumentKind.String && arguments[1].StringValue == "hazel", "string kind was not preserved");
        Assert(arguments[2].Kind == ReciteConditionArgumentKind.Integer && arguments[2].IntegerValue == 3, "integer kind was not preserved");
        Assert(arguments[3].Kind == ReciteConditionArgumentKind.Float && arguments[3].FloatValue == 1.5, "float kind was not preserved");
        Assert(arguments[4].Kind == ReciteConditionArgumentKind.Boolean && arguments[4].BooleanValue, "boolean kind was not preserved");
    }

    private static void RejectMalformedTypedConditionArguments()
    {
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[] { 0x90, 0x00 }), "trailing bytes");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[] { 0x00 }), "non-array root");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[]
        {
            0x91, 0x81, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d',
            0xa7, (byte)'b', (byte)'o', (byte)'o', (byte)'l', (byte)'e', (byte)'a', (byte)'n'
        }), "missing value");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[]
        {
            0x91, 0x83, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d',
            0xa7, (byte)'b', (byte)'o', (byte)'o', (byte)'l', (byte)'e', (byte)'a', (byte)'n',
            0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xc3,
            0xa5, (byte)'e', (byte)'x', (byte)'t', (byte)'r', (byte)'a', 0xc0
        }), "unknown field");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[]
        {
            0x91, 0x83, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d',
            0xa7, (byte)'b', (byte)'o', (byte)'o', (byte)'l', (byte)'e', (byte)'a', (byte)'n',
            0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xc3,
            0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xc2
        }), "duplicate field");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[]
        {
            0x91, 0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d',
            0xa7, (byte)'b', (byte)'o', (byte)'o', (byte)'l', (byte)'e', (byte)'a', (byte)'n',
            0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xa5, (byte)'t', (byte)'r', (byte)'u', (byte)'e'
        }), "wrong value type");
        ExpectFormatException(() => ReciteMessagePack.DecodeTypedConditionArgs(new byte[]
        {
            0x91, 0x82, 0xa4, (byte)'k', (byte)'i', (byte)'n', (byte)'d',
            0xa5, (byte)'f', (byte)'l', (byte)'o', (byte)'a', (byte)'t',
            0xa5, (byte)'v', (byte)'a', (byte)'l', (byte)'u', (byte)'e', 0xcb,
            0x7f, 0xf8, 0, 0, 0, 0, 0, 0
        }), "nonfinite value");
    }

    private static void RejectMalformedNestedOutput()
    {
        var bytes = BuildBatch(writer =>
        {
            WriteLine(writer);
            writer.WriteString("metadata");
            writer.WriteArrayHeader(1);
            writer.WriteBool(true);
        });

        ExpectValidation(() => ReciteDialogueService.DecodeBatchBytes(bytes), "nested output value");
    }

    private static void RejectUnknownTaggedScalarKind()
    {
        var bytes = BuildBatch(writer =>
        {
            WriteLine(writer);
            writer.WriteString("metadata");
            writer.WriteArrayHeader(1);
            writer.WriteMapHeader(2);
            writer.WriteString("key");
            writer.WriteString("colour");
            writer.WriteString("value");
            writer.WriteMapHeader(2);
            writer.WriteString("kind");
            writer.WriteString("colour");
            writer.WriteString("value");
            writer.WriteString("red");
        });

        ExpectValidation(() => ReciteDialogueService.DecodeBatchBytes(bytes), "unknown tagged scalar kind");
    }

    private static void RejectMalformedChoiceOutput()
    {
        var bytes = BuildBatch(writer =>
        {
            writer.WriteMapHeader(3);
            writer.WriteString("kind");
            writer.WriteString("prompt");
            writer.WriteString("line");
            writer.WriteNull();
            writer.WriteString("choices");
            writer.WriteArrayHeader(1);
            writer.WriteBool(false);
        });

        ExpectValidation(() => ReciteDialogueService.DecodeBatchBytes(bytes), "malformed choice output");
    }

    private static void PreserveRestoredBlockingRequestId()
    {
        var first = ReciteDialogueService.DecodeBatchBytes(BuildBatch(WriteBlockingEffect));
        var restored = ReciteDialogueService.DecodeBatchBytes(BuildBatch(WriteBlockingEffect));
        var firstEffect = GetEffect(first);
        var restoredEffect = GetEffect(restored);
        Assert(firstEffect.Effect.Id == restoredEffect.Effect.Id, "restored blocking request ID changed");
        Assert(firstEffect.Effect.Mode == "blocking", "fixture effect was not blocking");
    }

    private static void RejectInvalidConditionPointers()
    {
        ExpectFormatException(() => ReciteNativeBridge.ReadTypedConditionArgs(IntPtr.Zero, new UIntPtr(1)), "null pointer with nonzero length");
        ExpectFormatException(() => ReciteNativeBridge.ReadTypedConditionArgs(new IntPtr(1), UIntPtr.Zero), "nonnull pointer with zero length");
        ExpectFormatException(() => ReciteNativeBridge.ReadTypedConditionArgs(IntPtr.Zero, UIntPtr.Zero), "zero pointer and length");

        var payload = new byte[] { 0x90 };
        var handle = GCHandle.Alloc(payload, GCHandleType.Pinned);
        try
        {
            var arguments = ReciteNativeBridge.ReadTypedConditionArgs(handle.AddrOfPinnedObject(), new UIntPtr(1));
            Assert(arguments.Count == 0, "canonical empty condition args were not accepted");
        }
        finally
        {
            handle.Free();
        }
    }

    private static void RegisterTypedConditionApi()
    {
        using (var service = new ReciteDialogueService())
        {
            service.RegisterTypedCondition("has_item", args =>
                args.Count == 1 && args[0].Kind == ReciteConditionArgumentKind.Identifier);
            service.RegisterTypedConditionValue("mood", args =>
                ReciteConditionValue.Enum(args[0].StringValue));
            service.RegisterCondition("legacy", args => args.Count == 0);
        }
    }

    private static void PreserveEnumConditionResult()
    {
        using (var service = new ReciteDialogueService())
        {
            service.RegisterTypedConditionValue("mood", args =>
            {
                Assert(args.Count == 0, "enum condition arguments were not decoded");
                return ReciteConditionValue.Enum("calm");
            });

            var functionName = ReciteNativeBridge.ToUtf8NullTerminated("mood");
            var argumentBytes = new byte[] { 0x90 };
            var functionHandle = GCHandle.Alloc(functionName, GCHandleType.Pinned);
            var argumentHandle = GCHandle.Alloc(argumentBytes, GCHandleType.Pinned);
            try
            {
                var query = new ReciteNativeBridge.ReciteConditionQuery
                {
                    FunctionName = functionHandle.AddrOfPinnedObject(),
                    ArgsMsgpack = argumentHandle.AddrOfPinnedObject(),
                    ArgsLen = new UIntPtr((ulong)argumentBytes.Length)
                };
                var queryHandle = GCHandle.Alloc(query, GCHandleType.Pinned);
                try
                {
                    var result = service.EvaluateCondition(queryHandle.AddrOfPinnedObject(), IntPtr.Zero);
                    Assert(result.Ok == 1, "enum condition result was not successful");
                    Assert(result.ValueMsgpack != IntPtr.Zero, "enum condition result payload was null");
                    var actual = new byte[checked((int)result.ValueLen.ToUInt64())];
                    Marshal.Copy(result.ValueMsgpack, actual, 0, actual.Length);
                    AssertBytesEqual(
                        ReciteMessagePack.EncodeConditionEnum("calm"),
                        actual,
                        "enum condition result payload");
                }
                finally
                {
                    queryHandle.Free();
                }
            }
            finally
            {
                argumentHandle.Free();
                functionHandle.Free();
            }
        }
    }

    private static void PreserveConditionEnumStringBoundaries()
    {
        AssertConditionEnumVariant(new string('a', ushort.MaxValue), 0xda, ushort.MaxValue);
        AssertConditionEnumVariant(new string('b', ushort.MaxValue + 1), 0xdb, ushort.MaxValue + 1);
        // 32,768 two-byte UTF-8 characters are exactly 65,536 encoded bytes.
        AssertConditionEnumVariant(new string('\u00e9', 32768), 0xdb, 65536);
    }

    private static void AssertConditionEnumVariant(string expected, byte stringMarker, int expectedByteLength)
    {
        var encoded = ReciteMessagePack.EncodeConditionEnum(expected);
        var reader = new ReciteMessagePackReader(encoded);
        var map = reader.ReadMap();
        reader.EnsureEnd();
        Assert(map.TryGetValue("variant", out var value) && value is string && (string)value == expected,
            "condition enum variant did not round-trip");

        var markerIndex = FindVariantStringMarker(encoded);
        Assert(encoded[markerIndex] == stringMarker, "condition enum string used the wrong MessagePack width");
        if (stringMarker == 0xda)
        {
            var length = (encoded[markerIndex + 1] << 8) | encoded[markerIndex + 2];
            Assert(length == expectedByteLength, "str16 length was truncated");
        }
        else
        {
            var length = ((uint)encoded[markerIndex + 1] << 24)
                | ((uint)encoded[markerIndex + 2] << 16)
                | ((uint)encoded[markerIndex + 3] << 8)
                | encoded[markerIndex + 4];
            Assert(length == (uint)expectedByteLength, "str32 length was truncated");
        }
    }

    private static int FindVariantStringMarker(byte[] encoded)
    {
        for (var index = 0; index + 8 < encoded.Length; index++)
        {
            if (encoded[index] == 0xa7
                && encoded[index + 1] == (byte)'v'
                && encoded[index + 2] == (byte)'a'
                && encoded[index + 3] == (byte)'r'
                && encoded[index + 4] == (byte)'i'
                && encoded[index + 5] == (byte)'a'
                && encoded[index + 6] == (byte)'n'
                && encoded[index + 7] == (byte)'t')
            {
                return index + 8;
            }
        }

        throw new InvalidOperationException("condition enum variant key was not encoded");
    }

    private static byte[] BuildBatch(Action<ReciteMessagePackWriter> writeEvent)
    {
        var writer = new ReciteMessagePackWriter();
        writer.WriteMapHeader(2);
        writer.WriteString("batch_format_version");
        writer.WriteRaw(new byte[] { 0 });
        writer.WriteString("events");
        writer.WriteArrayHeader(1);
        writeEvent(writer);
        return writer.ToArray();
    }

    private static void WriteLine(ReciteMessagePackWriter writer)
    {
        writer.WriteMapHeader(6);
        writer.WriteString("kind");
        writer.WriteString("line");
        writer.WriteString("id");
        writer.WriteString("line-id");
        writer.WriteString("source_text");
        writer.WriteString("Line.");
        writer.WriteString("text");
        writer.WriteString("Line.");
        writer.WriteString("speaker");
        writer.WriteNull();
    }

    private static void WriteBlockingEffect(ReciteMessagePackWriter writer)
    {
        writer.WriteMapHeader(8);
        writer.WriteString("kind");
        writer.WriteString("effect");
        writer.WriteString("id");
        writer.WriteString("grant_item#1");
        writer.WriteString("mode");
        writer.WriteString("blocking");
        writer.WriteString("function");
        writer.WriteString("grant_item");
        writer.WriteString("args");
        writer.WriteArrayHeader(0);
        writer.WriteString("source_file");
        writer.WriteString("sample.recite");
        writer.WriteString("source_line");
        writer.WriteRaw(new byte[] { 1 });
        writer.WriteString("source_col");
        writer.WriteRaw(new byte[] { 1 });
    }

    private static ReciteEffectOutput GetEffect(ReciteOutputBatch batch)
    {
        Assert(batch.Events.Count == 1, "blocking effect fixture had unexpected output count");
        if (!(batch.Events[0] is ReciteEffectOutput effect))
        {
            throw new InvalidOperationException("blocking effect fixture did not decode as an effect");
        }

        return effect;
    }

    private static void PreserveSchemaMismatchStatus()
    {
        var error = new ReciteAdapterException(ReciteStatus.SchemaMismatch, "schema mismatch");
        Assert((int)error.Status == -4, "Unity schema mismatch status changed");
    }

    private static void ExpectFormatException(Action action, string name)
    {
        try
        {
            action();
        }
        catch (FormatException)
        {
            return;
        }

        throw new InvalidOperationException(name + " was accepted");
    }

    private static void ExpectValidation(Action action, string name)
    {
        try
        {
            action();
        }
        catch (ReciteAdapterException error)
        {
            Assert(error.Status == ReciteStatus.Validation, name + " was not projected as validation");
            return;
        }
        catch (Exception error)
        {
            throw new InvalidOperationException(name + " leaked " + error.GetType().Name, error);
        }

        throw new InvalidOperationException(name + " was accepted");
    }

    private static void AssertBytesEqual(byte[] expected, byte[] actual, string name)
    {
        Assert(expected.Length == actual.Length, name + " length changed");
        for (var index = 0; index < expected.Length; index++)
        {
            Assert(expected[index] == actual[index], name + " changed at byte " + index);
        }
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
