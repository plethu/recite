using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

namespace Recite.Unity.Editor
{
    // Unity declarations become standalone schema source. The Recite CLI owns
    // parsing, semantic validation, canonical JSON export and fingerprints.
    internal static class ReciteSchemaToml
    {
        internal static string Generate(ReciteSchemaRegistration source, string producerId)
        {
            if (source == null) throw new ArgumentNullException(nameof(source));
            var text = new StringBuilder("schema_version = 1\n\n[producer]\nid = ");
            text.Append(Quote(producerId)).Append("\n");
            foreach (var item in Ordered(source.enums, item => item.name))
            {
                Table(text, "types", item.name);
                text.Append("kind = \"enum\"\nvalues = ").Append(StringArray(item.values)).Append("\n");
            }
            foreach (var item in Ordered(source.registries, item => item.name))
            {
                Table(text, "registries", item.name);
                text.Append("values = ").Append(StringArray(item.values)).Append("\n");
            }
            foreach (var item in Ordered(source.speakers, item => item.name))
            {
                Table(text, "speakers", item.name);
                if (!string.IsNullOrEmpty(item.displayName))
                    text.Append("display_name = ").Append(Quote(item.displayName)).Append("\n");
            }
            foreach (var item in Ordered(source.conditions, item => item.name))
            {
                Table(text, "conditions", item.name);
                text.Append("returns = ").Append(Quote(item.returns)).Append("\nparams = ")
                    .Append(Parameters(item.parameters)).Append("\n");
            }
            foreach (var item in Ordered(source.effects, item => item.name))
            {
                Table(text, "effects", item.name);
                var modes = new List<string>();
                if (item.immediate) modes.Add("immediate");
                if (item.blocking) modes.Add("blocking");
                if (item.deferred) modes.Add("deferred");
                text.Append("modes = ").Append(StringArray(modes)).Append("\nparams = ")
                    .Append(Parameters(item.parameters)).Append("\n");
            }
            return text.ToString();
        }

        private static IEnumerable<T> Ordered<T>(T[] items, Func<T, string> name)
        {
            return (items ?? Array.Empty<T>()).OrderBy(name, StringComparer.Ordinal);
        }

        private static void Table(StringBuilder text, string group, string name)
        {
            text.Append("\n[").Append(group).Append('.').Append(Quote(name)).Append("]\n");
        }

        private static string Parameters(ReciteSchemaParameter[] parameters)
        {
            return "[" + string.Join(", ", (parameters ?? Array.Empty<ReciteSchemaParameter>())
                .Select(parameter => "{ name = " + Quote(parameter.name) + ", type = " + Quote(parameter.type) + " }")) + "]";
        }

        private static string StringArray(IEnumerable<string> values)
        {
            return "[" + string.Join(", ", (values ?? Array.Empty<string>()).Select(Quote)) + "]";
        }

        private static string Quote(string value)
        {
            if (value == null) return "\"\""; // The CLI reports invalid required names/types.
            var text = new StringBuilder("\"");
            foreach (var c in value)
            {
                switch (c)
                {
                    case '\\': text.Append("\\\\"); break;
                    case '"': text.Append("\\\""); break;
                    case '\n': text.Append("\\n"); break;
                    case '\r': text.Append("\\r"); break;
                    case '\t': text.Append("\\t"); break;
                    default:
                        if (c < 0x20 || c == 0x7f) text.Append("\\u").Append(((int)c).ToString("x4"));
                        else text.Append(c);
                        break;
                }
            }
            return text.Append('"').ToString();
        }
    }
}
