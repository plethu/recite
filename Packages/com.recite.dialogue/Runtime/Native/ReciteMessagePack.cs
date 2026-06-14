using System;
using System.Collections.Generic;

namespace Recite.Unity.Native
{
    internal static class ReciteMessagePack
    {
        internal static ReciteOutputBatch DecodeOutputBatch(byte[] bytes)
        {
            var reader = new ReciteMessagePackReader(bytes);
            var root = reader.ReadMap();
            var version = Convert.ToUInt16(root["batch_format_version"]);
            var rawEvents = (IReadOnlyList<object>)root["events"];
            var events = new List<ReciteOutput>(rawEvents.Count);
            foreach (var raw in rawEvents)
            {
                events.Add(ReadEvent((IReadOnlyDictionary<string, object>)raw));
            }

            return new ReciteOutputBatch(version, events);
        }

        internal static IReadOnlyList<object> DecodeConditionArgs(byte[] bytes)
        {
            var reader = new ReciteMessagePackReader(bytes);
            var rawArgs = reader.ReadArray();
            var args = new List<object>(rawArgs.Count);
            foreach (var raw in rawArgs)
            {
                args.Add(ReadTaggedScalar((IReadOnlyDictionary<string, object>)raw).Value);
            }

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
                    return new RecitePromptOutput(
                        map["line"] == null ? null : ReadLine((IReadOnlyDictionary<string, object>)map["line"]),
                        ReadList(map, "choices", ReadChoice));
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
                OptionalString(map, "speaker"),
                ReadList(map, "metadata", ReadMetadata));
        }

        private static ReciteChoice ReadChoice(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteChoice(
                RequiredString(map, "id"),
                RequiredString(map, "source_text"),
                RequiredString(map, "text"),
                ReadList(map, "metadata", ReadMetadata),
                ReadEcho((IReadOnlyDictionary<string, object>)map["echo"]),
                ReadAvailability((IReadOnlyDictionary<string, object>)map["availability"]));
        }

        private static ReciteChoiceEcho ReadEcho(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteChoiceEcho(RequiredString(map, "kind"), OptionalString(map, "explicit_line_id"));
        }

        private static ReciteChoiceAvailability ReadAvailability(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteChoiceAvailability(
                Convert.ToBoolean(map["is_available"]),
                map["primary_reason"] == null ? null : ReadReason((IReadOnlyDictionary<string, object>)map["primary_reason"]),
                map["reason_tree"] == null ? null : ReadReasonTree((IReadOnlyDictionary<string, object>)map["reason_tree"]));
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
            return new ReciteReasonArg(RequiredString(map, "name"), ReadTaggedScalar((IReadOnlyDictionary<string, object>)map["value"]));
        }

        private static ReciteEffect ReadEffect(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteEffect(
                RequiredString(map, "id"),
                RequiredString(map, "mode"),
                RequiredString(map, "function"),
                ReadList(map, "args", ReadTaggedScalar),
                RequiredString(map, "source_file"),
                Convert.ToUInt32(map["source_line"]),
                Convert.ToUInt32(map["source_col"]));
        }

        private static ReciteMetadata ReadMetadata(IReadOnlyDictionary<string, object> map)
        {
            return new ReciteMetadata(RequiredString(map, "key"), ReadTaggedScalar((IReadOnlyDictionary<string, object>)map["value"]));
        }

        private static ReciteTaggedValue ReadTaggedScalar(IReadOnlyDictionary<string, object> map)
        {
            var kind = RequiredString(map, "kind");
            if (kind == "array")
            {
                return new ReciteTaggedValue(kind, ReadList(map, "values", ReadTaggedScalar));
            }

            return new ReciteTaggedValue(kind, map.TryGetValue("value", out var value) ? value : null);
        }

        private static IReadOnlyList<T> ReadList<T>(IReadOnlyDictionary<string, object> map, string key, Func<IReadOnlyDictionary<string, object>, T> read)
        {
            var rawItems = (IReadOnlyList<object>)map[key];
            var items = new List<T>(rawItems.Count);
            foreach (var raw in rawItems)
            {
                items.Add(read((IReadOnlyDictionary<string, object>)raw));
            }

            return items;
        }

        private static string RequiredString(IReadOnlyDictionary<string, object> map, string key)
        {
            return (string)map[key];
        }

        private static string OptionalString(IReadOnlyDictionary<string, object> map, string key)
        {
            return map.TryGetValue(key, out var value) ? value as string : null;
        }

    }
}
