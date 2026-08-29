using System;
using System.Collections.Generic;
using System.IO;
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
            PreserveTypedInterpolationValues();
            PreserveLocaleCatalogFallbacks();
            NativeTraversalThroughRawBridge();
            RejectInvalidLocaleStringsAndPluralCounts();
            EndFreesLocaleCallbackAllocations();
            PreservePluralOutputTrace();
            PreserveLegacyNonPluralLine();
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

    private static void PreserveTypedInterpolationValues()
    {
        Assert(ReciteNativeBridge.AbiMajor == 0, "FFI ABI major version changed");
        Assert(ReciteNativeBridge.AbiMinor == 5, "locale provider ABI minor version changed");
        Assert(ReciteNativeBridge.AbiPatch == 0, "FFI ABI patch version changed");

        var values = new List<ReciteInterpolationValue>
        {
            ReciteInterpolationValue.String("name", "Ada"),
            ReciteInterpolationValue.Integer("count", 2),
            ReciteInterpolationValue.Float("ratio", 1.5),
            ReciteInterpolationValue.Boolean("ready", true)
        };
        using (var buffer = new ReciteNativeBridge.InterpolationValueBuffer(values))
        {
            Assert(buffer.Pointer != IntPtr.Zero, "typed interpolation records were not allocated");
            Assert(buffer.Length.ToUInt64() == 4, "typed interpolation record count changed");
        }
        using (var disposedBuffer = new ReciteNativeBridge.InterpolationValueBuffer(values))
        {
            disposedBuffer.Dispose();
            Assert(disposedBuffer.Pointer == IntPtr.Zero, "typed interpolation records outlived disposal");
        }

        ExpectArgumentException(
            () => ReciteInterpolationValue.Float("ratio", double.NaN),
            "nonfinite interpolation float");
        ExpectArgumentException(
            () => ReciteInterpolationValue.String("bad\0name", "Ada"),
            "embedded NUL in interpolation name");
        ExpectArgumentException(
            () => ReciteInterpolationValue.String("name", "Ada\0"),
            "embedded NUL in interpolation string value");
    }

    private static void PreserveLocaleCatalogFallbacks()
    {
        var catalog = new ReciteLocaleCatalog();
        catalog.SetPluralRule("fr", "nplurals=2; plural=(n != 1);");
        catalog.AddTranslation("fr", "line-id", "Hello {name}.", "Bonjour {name}.");
        catalog.AddChoiceTranslation("fr", "choice-id", "Choose this.", "Choisir ceci.");
        catalog.AddPluralTranslation(
            "fr",
            "letters-id",
            "You have one letter.",
            "You have {count} letters.",
            new[] { "Vous avez une lettre.", "Vous avez {count} lettres." });

        Assert(
            catalog.Lookup("line-id", "Hello {name}.", ReciteLocaleTextDomain.Line, "fr-CA", null) == "Bonjour {name}.",
            "locale fallback did not use the language catalogue");
        Assert(
            catalog.Lookup("choice-id", "Choose this.", ReciteLocaleTextDomain.Choice, "fr-CA", null) == "Choisir ceci.",
            "choice catalogue translation was not preserved");
        var plural = catalog.ResolvePlural(
            "letters-id",
            "You have one letter.",
            "You have {count} letters.",
            2,
            ReciteLocaleTextDomain.Line,
            "fr-CA",
            null);
        Assert(plural.Text == "Vous avez {count} lettres." && plural.SelectedArm == 1, "plural locale fallback changed the selected arm");
        Assert(catalog.Lookup("missing", "Source.", ReciteLocaleTextDomain.Line, "fr-CA", null) == null, "missing translation did not request source fallback");
        catalog.AddTranslation(" fr ", "preserved-locale", "Source.", "Conserve.");
        Assert(
            catalog.Lookup("preserved-locale", "Source.", ReciteLocaleTextDomain.Line, " fr ", null) == "Conserve.",
            "valid locale was trimmed into a different catalogue key");

        ExpectArgumentException(
            () => catalog.AddTranslation("fr", "bad\0id", "Source.", "Translation."),
            "locale catalogue embedded NUL");
        ExpectArgumentException(
            () => catalog.AddTranslation(" \t", "blank-locale", "Source.", "Translation."),
            "locale catalogue whitespace-only locale");
        ExpectArgumentException(
            () => catalog.SetPluralRule("\u2003", "nplurals=2; plural=(n != 1);"),
            "locale catalogue whitespace-only plural rule locale");
        catalog.AddTranslation("fr", "conflict-id", "Source.", "Traduction.");
        ExpectArgumentException(
            () => catalog.AddTranslation("fr", "conflict-id", "Source.", "Autre traduction."),
            "conflicting locale catalogue entry");

        using (var service = new ReciteDialogueService())
        {
            ExpectArgumentException(
                () => service.Start(new ReciteDialogueAsset(Array.Empty<byte>()), locale: " \t"),
                "dialogue start whitespace-only locale");
        }
    }

    private static readonly ReciteNativeBridge.ReciteLocaleFn NativeFallbackLocaleCallback = NativeFallbackLocale;
    private static readonly ReciteNativeBridge.ReciteConditionFn NativeFalseConditionCallback = NativeFalseCondition;
    private static int nativeLocaleCallbackCalls;
    private static GCHandle nativeConditionPayload;

    private static ReciteNativeBridge.ReciteConditionResult NativeFalseCondition(IntPtr query, IntPtr userdata)
    {
        return new ReciteNativeBridge.ReciteConditionResult
        {
            Ok = 1,
            ValueMsgpack = nativeConditionPayload.AddrOfPinnedObject(),
            ValueLen = new UIntPtr((ulong)((byte[])nativeConditionPayload.Target).Length)
        };
    }

    private static ReciteNativeBridge.ReciteLocaleResult NativeFallbackLocale(IntPtr query, IntPtr userdata)
    {
        nativeLocaleCallbackCalls++;
        return new ReciteNativeBridge.ReciteLocaleResult
        {
            Ok = 1,
            SelectedArm = -1
        };
    }

    private static void NativeTraversalThroughRawBridge()
    {
        var samplePath = Environment.GetEnvironmentVariable("RECITE_UNITY_SAMPLE_ASSET");
        Assert(!string.IsNullOrEmpty(samplePath) && File.Exists(samplePath),
            "native traversal fixture path was not configured");

        var assetBytes = File.ReadAllBytes(samplePath);
        nativeLocaleCallbackCalls = 0;
        var conditionPayload = ReciteMessagePack.EncodeConditionBool(false);
        nativeConditionPayload = GCHandle.Alloc(conditionPayload, GCHandleType.Pinned);
        ulong assetHandle = 0;
        ulong sessionHandle = 0;
        try
        {
            var status = ReciteNativeBridge.AssetLoad(
                assetBytes,
                new UIntPtr((ulong)assetBytes.Length),
                out assetHandle);
            Assert(status == ReciteStatus.Ok, "native traversal fixture did not load");

            status = ReciteNativeBridge.SessionCreate(
                assetHandle,
                null,
                ReciteNativeBridge.ToUtf8NullTerminated("fr-CA"),
                out sessionHandle);
            Assert(status == ReciteStatus.Ok,
                "native session creation failed: " + status + " " + ReciteNativeBridge.LastErrorMessage());
            status = ReciteNativeBridge.SessionSetLocaleProvider(
                sessionHandle, NativeFallbackLocaleCallback, IntPtr.Zero);
            Assert(status == ReciteStatus.Ok, "native locale provider installation failed");
            status = ReciteNativeBridge.SessionSetLocaleVariant(
                sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated("formal"));
            Assert(status == ReciteStatus.Ok, "native variant installation failed");
            status = ReciteNativeBridge.SessionRegisterCondition(
                sessionHandle,
                ReciteNativeBridge.ToUtf8NullTerminated("has_key"),
                NativeFalseConditionCallback,
                IntPtr.Zero);
            Assert(status == ReciteStatus.Ok, "native condition installation failed");
            status = ReciteNativeBridge.SessionBegin(sessionHandle, out var nativeBatch);
            Assert(status == ReciteStatus.Ok,
                "native start with locale provider failed: " + status + " " + ReciteNativeBridge.LastErrorMessage());
            var initial = ReciteDialogueService.DecodeBatchBytes(
                ReciteNativeBridge.CopyAndFree(ref nativeBatch));
            RecitePromptOutput prompt = null;
            foreach (var output in initial.Events)
            {
                if (output is RecitePromptOutput candidate)
                {
                    prompt = candidate;
                    break;
                }
            }
            Assert(prompt != null && prompt.Choices.Count > 0, "native start did not reach a prompt");
            Assert(nativeLocaleCallbackCalls > 0, "native start did not invoke the locale callback");

            status = ReciteNativeBridge.SessionSnapshot(sessionHandle, out var nativeSnapshot);
            Assert(status == ReciteStatus.Ok, "native snapshot failed");
            var snapshot = ReciteNativeBridge.CopyAndFree(ref nativeSnapshot);
            ReciteNativeBridge.SessionFree(sessionHandle);
            sessionHandle = 0;

            status = ReciteNativeBridge.SessionRestoreWithValuesAndLocaleProviderAndVariant(
                assetHandle,
                snapshot,
                new UIntPtr((ulong)snapshot.Length),
                IntPtr.Zero,
                UIntPtr.Zero,
                ReciteNativeBridge.ToUtf8NullTerminated("formal"),
                NativeFallbackLocaleCallback,
                IntPtr.Zero,
                out sessionHandle,
                out nativeBatch);
            Assert(status == ReciteStatus.Ok, "native restore with locale provider failed");
            var restored = ReciteNativeBridge.CopyAndFree(ref nativeBatch);
            Assert(ReciteDialogueService.DecodeBatchBytes(restored).Events.Count == 0,
                "native restore changed the pending prompt");

            status = ReciteNativeBridge.SessionChoose(
                sessionHandle,
                ReciteNativeBridge.ToUtf8NullTerminated(prompt.Choices[0].Id),
                out nativeBatch);
            Assert(status == ReciteStatus.Ok, "native choice traversal failed");
            var chosen = ReciteDialogueService.DecodeBatchBytes(
                ReciteNativeBridge.CopyAndFree(ref nativeBatch));
            ReciteEffect effect = null;
            foreach (var output in chosen.Events)
            {
                if (output is ReciteEffectOutput effectOutput)
                {
                    effect = effectOutput.Effect;
                    break;
                }
            }
            Assert(effect != null && effect.Mode == "blocking",
                "native choice traversal did not reach the blocking effect");

            status = ReciteNativeBridge.SessionAcknowledgeEffect(
                sessionHandle,
                ReciteNativeBridge.ToUtf8NullTerminated(effect.Id),
                1,
                null,
                out nativeBatch);
            Assert(status == ReciteStatus.Ok, "native acknowledgement traversal failed");
            var acknowledged = ReciteDialogueService.DecodeBatchBytes(
                ReciteNativeBridge.CopyAndFree(ref nativeBatch));
            Assert(acknowledged.Events.Count > 0
                && acknowledged.Events[acknowledged.Events.Count - 1] is ReciteEndOutput,
                "native acknowledgement did not finish the sample traversal");
        }
        finally
        {
            if (sessionHandle != 0)
            {
                ReciteNativeBridge.SessionFree(sessionHandle);
            }
            if (assetHandle != 0)
            {
                ReciteNativeBridge.AssetFree(assetHandle);
            }
            if (nativeConditionPayload.IsAllocated)
            {
                nativeConditionPayload.Free();
            }
        }
    }

    private static void RejectInvalidLocaleStringsAndPluralCounts()
    {
        ExpectArgumentException(
            () => ReciteNativeBridge.ToUtf8NullTerminated("bad\0value"),
            "native embedded NUL");
        ExpectArgumentException(
            () => ReciteNativeBridge.ToUtf8NullTerminated("bad\ud800value"),
            "native unpaired surrogate");

        var catalog = new ReciteLocaleCatalog();
        ExpectArgumentException(
            () => catalog.AddTranslation("fr", "id", "Source", "bad\udffftranslation"),
            "catalogue unpaired surrogate");
        try
        {
            catalog.SetPluralRule("fr", "nplurals=2; plural=(n == 42 ? 2 : 0);");
        }
        catch (ReciteAdapterException)
        {
            return;
        }

        throw new InvalidOperationException("reachable invalid plural arm was accepted");
    }

    private static void EndFreesLocaleCallbackAllocations()
    {
        var service = new ReciteDialogueService();
        var catalog = new ReciteLocaleCatalog();
        catalog.AddTranslation("fr", "line-id", "Source.", "Traduction.");
        var catalogField = typeof(ReciteDialogueService).GetField(
            "localeCatalog",
            System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
        catalogField.SetValue(service, catalog);

        var id = Marshal.StringToCoTaskMemUTF8("line-id");
        var source = Marshal.StringToCoTaskMemUTF8("Source.");
        var locale = Marshal.StringToCoTaskMemUTF8("fr");
        var query = Marshal.AllocHGlobal(Marshal.SizeOf<ReciteNativeBridge.ReciteLocaleQuery>());
        try
        {
            Marshal.StructureToPtr(new ReciteNativeBridge.ReciteLocaleQuery
            {
                Kind = 0,
                Id = id,
                SourceText = source,
                PluralSourceText = IntPtr.Zero,
                Count = -1,
                Domain = 0,
                Locale = locale,
                Variant = IntPtr.Zero
            }, query, false);
            service.EvaluateLocale(query, IntPtr.Zero);
            var count = (int)typeof(ReciteDialogueService)
                .GetProperty("LocaleCallbackAllocationCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
                .GetValue(service);
            Assert(count > 0, "locale callback did not allocate a result");
            service.End();
            count = (int)typeof(ReciteDialogueService)
                .GetProperty("LocaleCallbackAllocationCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)
                .GetValue(service);
            Assert(count == 0, "End did not free locale callback allocations");
        }
        finally
        {
            Marshal.FreeHGlobal(query);
            Marshal.FreeCoTaskMem(id);
            Marshal.FreeCoTaskMem(source);
            Marshal.FreeCoTaskMem(locale);
            service.Dispose();
        }
    }

    private static void PreservePluralOutputTrace()
    {
        var bytes = BuildBatch(writer =>
        {
            writer.WriteMapHeader(7);
            writer.WriteString("kind");
            writer.WriteString("line");
            writer.WriteString("id");
            writer.WriteString("letters-id");
            writer.WriteString("source_text");
            writer.WriteString("You have {count} letters.");
            writer.WriteString("text");
            writer.WriteString("Vous avez {count} lettres.");
            writer.WriteString("speaker");
            writer.WriteNull();
            writer.WriteString("metadata");
            writer.WriteArrayHeader(0);
            writer.WriteString("plural");
            writer.WriteMapHeader(5);
            writer.WriteString("singular_source_text");
            writer.WriteString("You have one letter.");
            writer.WriteString("plural_source_text");
            writer.WriteString("You have {count} letters.");
            writer.WriteString("count");
            writer.WriteRaw(new byte[] { 2 });
            writer.WriteString("selected_arm");
            writer.WriteRaw(new byte[] { 1 });
            writer.WriteString("resolution");
            writer.WriteMapHeader(7);
            writer.WriteString("attempts");
            writer.WriteArrayHeader(1);
            writer.WriteMapHeader(5);
            writer.WriteString("locale");
            writer.WriteString("fr");
            writer.WriteString("context");
            writer.WriteString("letters-id");
            writer.WriteString("key");
            writer.WriteString("letters-id");
            writer.WriteString("selected_arm");
            writer.WriteRaw(new byte[] { 1 });
            writer.WriteString("outcome");
            writer.WriteString("matched");
            writer.WriteString("matched_locale");
            writer.WriteString("fr");
            writer.WriteString("matched_context");
            writer.WriteString("letters-id");
            writer.WriteString("matched_key");
            writer.WriteString("letters-id");
            writer.WriteString("matched_arm");
            writer.WriteRaw(new byte[] { 1 });
            writer.WriteString("source_fallback_arm");
            writer.WriteNull();
            writer.WriteString("outcome");
            writer.WriteString("translated");
        });
        var batch = ReciteMessagePack.DecodeOutputBatch(bytes);
        var line = ((ReciteLineOutput)batch.Events[0]).Line;
        Assert(line.Plural != null, "plural output metadata was discarded");
        Assert(line.Plural.Resolution.Outcome == "translated", "plural output outcome changed");
        Assert(line.Plural.Resolution.Attempts.Count == 1 && line.Plural.Resolution.Attempts[0].Outcome == "matched", "plural output attempts changed");
    }

    private static void PreserveLegacyNonPluralLine()
    {
        var bytes = BuildBatch(writer =>
        {
            WriteLine(writer);
            writer.WriteString("metadata");
            writer.WriteArrayHeader(0);
        });
        var batch = ReciteMessagePack.DecodeOutputBatch(bytes);
        var line = ((ReciteLineOutput)batch.Events[0]).Line;
        Assert(line.Plural == null, "legacy non-plural line acquired plural metadata");
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

    private static void ExpectArgumentException(Action action, string name)
    {
        try
        {
            action();
        }
        catch (ArgumentException)
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
