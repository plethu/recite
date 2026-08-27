using System;
using System.Collections.Generic;
using Recite.Unity;

namespace Recite.Unity.Native
{
    internal static class ReciteMessagePack
    {
        internal static ReciteOutputBatch DecodeOutputBatch(byte[] bytes)
        {
            var reader = new ReciteMessagePackReader(bytes);
            var root = reader.ReadMap();
            reader.EnsureEnd();
            var version = RequiredUInt16(root, "batch_format_version");
            if (version != 0)
            {
                throw new FormatException("unsupported Recite batch format version: " + version);
            }

            var rawEvents = RequiredArray(root, "events");
            var events = new List<ReciteOutput>(rawEvents.Count);
            foreach (var raw in rawEvents)
            {
                events.Add(ReadEvent(RequiredMapValue(raw, "events")));
            }

            return new ReciteOutputBatch(version, events);
        }

        internal static IReadOnlyList<object> DecodeConditionArgs(byte[] bytes)
        {
            var typed = DecodeTypedConditionArgs(bytes);
            var args = new List<object>(typed.Count);
            foreach (var argument in typed)
            {
                args.Add(argument.LegacyValue);
            }

            return args;
        }

        internal static IReadOnlyList<ReciteConditionArgument> DecodeTypedConditionArgs(byte[] bytes)
        {
            var reader = new ReciteMessagePackReader(bytes);
            var rawArgs = reader.ReadArray();
            var args = new List<ReciteConditionArgument>(rawArgs.Count);
            foreach (var raw in rawArgs)
            {
                if (!(raw is IReadOnlyDictionary<string, object> map))
                {
                    throw new FormatException("condition argument must be a MessagePack map");
                }

                args.Add(ReadConditionArgument(map));
            }
            reader.EnsureEnd();

            return args;
        }

        internal static byte[] EncodeConditionBool(bool value)
        {
            var writer = new ReciteMessagePackWriter();
            writer.WriteMapHeader(2);
            writer.WriteString("kind");
            writer.WriteString("bool");
            writer.WriteString("value");
            writer.WriteBool(value);
            return writer.ToArray();
        }

        internal static byte[] EncodeConditionEnum(string variant)
        {
            var writer = new ReciteMessagePackWriter();
            writer.WriteMapHeader(2);
            writer.WriteString("kind");
            writer.WriteString("enum");
            writer.WriteString("variant");
            writer.WriteString(variant ?? string.Empty);
            return writer.ToArray();
        }

        private static ReciteOutput ReadEvent(IReadOnlyDictionary<string, object> map)
        {
            var kind = RequiredString(map, "kind");
            switch (kind)
            {
                case "line":
                    return new ReciteLineOutput(ReadLine(map));
                case "prompt":
                {
                    var line = RequiredNullableMap(map, "line");
                    return new RecitePromptOutput(
                        line == null ? null : ReadLine(line),
                        ReadList(map, "choices", ReadChoice));
                }
                case "effect":
                    return new ReciteEffectOutput(ReadEffect(map));
                case "end":
                    return new ReciteEndOutput(ReadList(map, "deferred_effects", ReadEffect));
                default:
                    throw new FormatException("unknown Recite FFI event kind: " + kind);
            }
        }

        private static ReciteLine ReadLine(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteLine(
                RequiredString(map, "id"),
                RequiredString(map, "source_text"),
                RequiredString(map, "text"),
                RequiredNullableString(map, "speaker"),
                ReadList(map, "metadata", ReadMetadata));
        }

        private static ReciteChoice ReadChoice(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteChoice(
                RequiredString(map, "id"),
                RequiredString(map, "source_text"),
                RequiredString(map, "text"),
                ReadList(map, "metadata", ReadMetadata),
                ReadEcho(RequiredMap(map, "echo")),
                ReadAvailability(RequiredMap(map, "availability")));
        }

        private static ReciteChoiceEcho ReadEcho(IReadOnlyDictionary<string, object> map)
        {
            var kind = RequiredString(map, "kind");
            if (kind != "none" && kind != "selected_text" && kind != "explicit_line")
            {
                throw new FormatException("unknown Recite choice echo kind: " + kind);
            }

            var explicitLineId = RequiredNullableString(map, "explicit_line_id");
            if ((kind == "explicit_line") != (explicitLineId != null))
            {
                throw new FormatException("Recite choice echo kind and explicit line ID do not agree");
            }

            return new ReciteChoiceEcho(kind, explicitLineId);
        }

        private static ReciteChoiceAvailability ReadAvailability(IReadOnlyDictionary<string, object> map)
        {
            var isAvailable = RequiredBool(map, "is_available");
            var primaryReason = RequiredNullableMap(map, "primary_reason");
            var reasonTree = RequiredNullableMap(map, "reason_tree");
            if (isAvailable && (primaryReason != null || reasonTree != null))
            {
                throw new FormatException("available Recite choice cannot contain availability reasons");
            }

            return new ReciteChoiceAvailability(
                isAvailable,
                primaryReason == null ? null : ReadReason(primaryReason),
                reasonTree == null ? null : ReadReasonTree(reasonTree));
        }

        private static ReciteAvailabilityReasonTree ReadReasonTree(IReadOnlyDictionary<string, object> map)
        {
            var kind = RequiredString(map, "kind");
            switch (kind)
            {
                case "all":
                case "any":
                    return new ReciteAvailabilityReasonTree(kind, ReadList(map, "children", ReadReasonTree), null, null);
                case "reason":
                    return new ReciteAvailabilityReasonTree(kind, Array.Empty<ReciteAvailabilityReasonTree>(), ReadReason(map), null);
                case "requirement_source_text":
                    return new ReciteAvailabilityReasonTree(kind, Array.Empty<ReciteAvailabilityReasonTree>(), null, RequiredString(map, "text"));
                default:
                    throw new FormatException("unknown Recite reason tree kind: " + kind);
            }
        }

        private static ReciteAvailabilityReason ReadReason(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteAvailabilityReason(
                RequiredString(map, "id"),
                RequiredString(map, "source_text"),
                RequiredString(map, "text"),
                ReadList(map, "args", ReadReasonArg));
        }

        private static ReciteReasonArg ReadReasonArg(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteReasonArg(RequiredString(map, "name"), ReadTaggedValue(RequiredMap(map, "value"), false, true));
        }

        private static ReciteEffect ReadEffect(IReadOnlyDictionary<string, object> map)
        {
            var mode = RequiredString(map, "mode");
            if (mode != "deferred" && mode != "immediate" && mode != "blocking")
            {
                throw new FormatException("unknown Recite effect mode: " + mode);
            }

            return new ReciteEffect(
                RequiredString(map, "id"),
                mode,
                RequiredString(map, "function"),
                ReadList(map, "args", value => ReadTaggedValue(value, false, true)),
                RequiredString(map, "source_file"),
                RequiredUInt32(map, "source_line"),
                RequiredUInt32(map, "source_col"));
        }

        private static ReciteMetadata ReadMetadata(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteMetadata(RequiredString(map, "key"), ReadTaggedValue(RequiredMap(map, "value"), true, false));
        }

        private static ReciteTaggedValue ReadTaggedValue(
            IReadOnlyDictionary<string, object> map,
            bool allowArray,
            bool allowIdentifier)
        {
            var kind = RequiredString(map, "kind");
            switch (kind)
            {
                case "string":
                    EnsureExactKeys(map, "kind", "value");
                    return new ReciteTaggedValue(kind, RequiredString(map, "value"));
                case "integer":
                    EnsureExactKeys(map, "kind", "value");
                    return new ReciteTaggedValue(kind, RequiredInt64(map, "value"));
                case "float":
                    EnsureExactKeys(map, "kind", "value");
                    var floatValue = RequiredDouble(map, "value");
                    if (double.IsNaN(floatValue) || double.IsInfinity(floatValue))
                    {
                        throw new FormatException("Recite tagged float must be finite");
                    }
                    return new ReciteTaggedValue(kind, floatValue);
                case "boolean":
                    EnsureExactKeys(map, "kind", "value");
                    return new ReciteTaggedValue(kind, RequiredBool(map, "value"));
                case "identifier":
                    if (!allowIdentifier)
                    {
                        throw new FormatException("Recite tagged value kind `identifier` is not valid here");
                    }
                    EnsureExactKeys(map, "kind", "value");
                    return new ReciteTaggedValue(kind, RequiredString(map, "value"));
                case "array":
                    if (!allowArray)
                    {
                        throw new FormatException("Recite tagged value kind `array` is not valid here");
                    }
                    EnsureExactKeys(map, "kind", "values");
                    return new ReciteTaggedValue(kind, ReadList(map, "values", value => ReadTaggedValue(value, false, false)));
                default:
                    throw new FormatException("unknown Recite tagged value kind: " + kind);
            }
        }

        private static ReciteConditionArgument ReadConditionArgument(IReadOnlyDictionary<string, object> map)
        {
            EnsureExactKeys(map, "kind", "value");
            var kind = RequiredString(map, "kind");
            switch (kind)
            {
                case "identifier":
                    return ReciteConditionArgument.Identifier(RequiredString(map, "value"));
                case "string":
                    return ReciteConditionArgument.String(RequiredString(map, "value"));
                case "integer":
                    return ReciteConditionArgument.Integer(RequiredInt64(map, "value"));
                case "float":
                    var floatValue = RequiredDouble(map, "value");
                    if (double.IsNaN(floatValue) || double.IsInfinity(floatValue))
                    {
                        throw new FormatException("condition float argument must be finite");
                    }
                    return ReciteConditionArgument.Float(floatValue);
                case "boolean":
                    return ReciteConditionArgument.Boolean(RequiredBool(map, "value"));
                default:
                    throw new FormatException("unknown Recite condition argument kind: " + kind);
            }
        }

        private static IReadOnlyList<T> ReadList<T>(IReadOnlyDictionary<string, object> map, string key, Func<IReadOnlyDictionary<string, object>, T> read)
        {
            var rawItems = RequiredArray(map, key);
            var items = new List<T>(rawItems.Count);
            foreach (var raw in rawItems)
            {
                items.Add(read(RequiredMapValue(raw, key)));
            }

            return items;
        }

        private static IReadOnlyList<object> RequiredArray(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is IReadOnlyList<object> array))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be an array");
            }

            return array;
        }

        private static string RequiredString(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is string text))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a string");
            }

            return text;
        }

        private static long RequiredInt64(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is long integer))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be an integer");
            }

            return integer;
        }

        private static uint RequiredUInt32(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is long integer) || integer < 0 || integer > uint.MaxValue)
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a uint32");
            }

            return (uint)integer;
        }

        private static ushort RequiredUInt16(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is long integer) || integer < 0 || integer > ushort.MaxValue)
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a uint16");
            }

            return (ushort)integer;
        }

        private static double RequiredDouble(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is double number))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a float64");
            }

            return number;
        }

        private static bool RequiredBool(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value) || !(value is bool boolean))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a boolean");
            }

            return boolean;
        }

        private static IReadOnlyDictionary<string, object> RequiredMap(
            IReadOnlyDictionary<string, object> map,
            string key)
        {
            if (!map.TryGetValue(key, out var value))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` is required");
            }

            return RequiredMapValue(value, key);
        }

        private static IReadOnlyDictionary<string, object> RequiredMapValue(object value, string key)
        {
            if (!(value is IReadOnlyDictionary<string, object> nested))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a map");
            }

            return nested;
        }

        private static IReadOnlyDictionary<string, object> RequiredNullableMap(
            IReadOnlyDictionary<string, object> map,
            string key)
        {
            if (!map.TryGetValue(key, out var value))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` is required");
            }

            if (value == null)
            {
                return null;
            }

            return RequiredMapValue(value, key);
        }

        private static void EnsureExactKeys(IReadOnlyDictionary<string, object> map, string first, string second)
        {
            if (map.Count != 2 || !map.ContainsKey(first) || !map.ContainsKey(second))
            {
                throw new FormatException("condition argument must contain exactly `kind` and `value` fields");
            }
        }

        private static string RequiredNullableString(IReadOnlyDictionary<string, object> map, string key)
        {
            if (!map.TryGetValue(key, out var value))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` is required");
            }

            if (value == null)
            {
                return null;
            }

            if (!(value is string text))
            {
                throw new FormatException("Recite MessagePack field `" + key + "` must be a string or null");
            }

            return text;
        }

    }
}
