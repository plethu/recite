using System;
using System.Collections.Generic;
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
            RegisterTypedConditionApi();
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

        ExpectFormatException(() => ReciteMessagePack.DecodeOutputBatch(bytes), "unsupported batch version");
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

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
