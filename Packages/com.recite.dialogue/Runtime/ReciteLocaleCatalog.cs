using System;
using System.Collections.Generic;
using Recite.Unity.Native;

namespace Recite.Unity
{
    public enum ReciteLocaleTextDomain : uint
    {
        Line,
        Choice,
        AvailabilityReason,
        PresentationLabel
    }

    /// <summary>
    /// Caller-owned dialogue translations and gettext-style plural rules.
    /// Entries are copied by <see cref="ReciteDialogueService.SetLocaleCatalog"/>
    /// before a native session uses them.
    /// </summary>
    public sealed class ReciteLocaleCatalog
    {
        private readonly Dictionary<CatalogKey, string[]> translations = new Dictionary<CatalogKey, string[]>(CatalogKeyComparer.Instance);
        private readonly Dictionary<string, PluralRule> pluralRules = new Dictionary<string, PluralRule>(StringComparer.Ordinal);

        public void AddTranslation(string locale, string id, string sourceText, string translation, ReciteLocaleTextDomain domain = ReciteLocaleTextDomain.Line, string variant = null)
        {
            RequireText(id, nameof(id));
            RequireText(sourceText, nameof(sourceText));
            if (variant != null) ReciteStringValidation.Validate(variant, nameof(variant));
            var key = CatalogKey.Singular(ValidateLocale(locale), domain, id, sourceText, variant);
            var validatedTranslation = ValidateString(translation, nameof(translation));
            ValidateTranslation(sourceText, validatedTranslation);
            AddEntry(key, new[] { validatedTranslation });
        }

        public void AddChoiceTranslation(string locale, string id, string sourceText, string translation, string variant = null)
        {
            AddTranslation(locale, id, sourceText, translation, ReciteLocaleTextDomain.Choice, variant);
        }

        public void AddPluralTranslation(string locale, string id, string sourceSingular, string sourcePlural, IReadOnlyList<string> translations, string variant = null)
        {
            var validatedLocale = ValidateLocale(locale);
            RequireText(id, nameof(id));
            RequireText(sourceSingular, nameof(sourceSingular));
            RequireText(sourcePlural, nameof(sourcePlural));
            ReciteStringValidation.Validate(id, nameof(id));
            ReciteStringValidation.Validate(sourceSingular, nameof(sourceSingular));
            ReciteStringValidation.Validate(sourcePlural, nameof(sourcePlural));
            if (variant != null) ReciteStringValidation.Validate(variant, nameof(variant));
            if (translations == null || translations.Count == 0)
            {
                throw new ArgumentException("plural translations must contain at least one arm", nameof(translations));
            }

            if (!pluralRules.TryGetValue(validatedLocale, out var rule))
            {
                throw new InvalidOperationException("a plural rule must be installed before plural translations");
            }
            if (translations.Count != rule.ArmCount)
            {
                throw new ArgumentException("plural translations must contain exactly the rule's nplurals arms", nameof(translations));
            }

            var arms = new string[translations.Count];
            for (var index = 0; index < translations.Count; index++)
            {
                arms[index] = ValidateString(translations[index], nameof(translations));
                ValidateTranslation(index == 0 ? sourceSingular : sourcePlural, arms[index]);
            }

            AddEntry(CatalogKey.Plural(validatedLocale, id, sourceSingular, sourcePlural, variant), arms);
        }

        /// <summary>
        /// Installs a complete gettext rule. Arm validation and selection are
        /// delegated to Recite's native core; callers cannot provide a second
        /// managed plural selector.
        /// </summary>
        public void SetPluralRule(string locale, string pluralForms)
        {
            var validatedLocale = ValidateLocale(locale);
            ReciteStringValidation.Validate(pluralForms, nameof(pluralForms));

            var armCount = ReciteNativeBridge.ValidatePluralRule(pluralForms);
            foreach (var entry in translations)
            {
                if (entry.Key.Locale == validatedLocale && entry.Key.PluralSourceText != null
                    && entry.Value.Length != armCount)
                {
                    throw new ArgumentException("plural translations must contain exactly the rule's nplurals arms", nameof(pluralForms));
                }
            }
            pluralRules[validatedLocale] = new PluralRule(armCount, pluralForms);
        }

        internal void ValidatePluralRules(Action<string, int, string> validator)
        {
            if (validator == null) throw new ArgumentNullException(nameof(validator));
            foreach (var pair in pluralRules)
            {
                validator(pair.Key, pair.Value.ArmCount, pair.Value.Header);
            }
        }

        internal ReciteLocaleCatalog Clone()
        {
            var clone = new ReciteLocaleCatalog();
            foreach (var entry in translations)
            {
                clone.translations.Add(entry.Key, (string[])entry.Value.Clone());
            }
            foreach (var rule in pluralRules)
            {
                clone.pluralRules.Add(rule.Key, rule.Value);
            }
            return clone;
        }

        internal string Lookup(string id, string sourceText, ReciteLocaleTextDomain domain, string locale, string variant)
        {
            var context = Context(id, domain);
            foreach (var candidateContext in Contexts(context, variant))
            {
                foreach (var candidateLocale in LocaleFallbacks(locale))
                {
                    var key = CatalogKey.Singular(candidateLocale, domain, id, sourceText, candidateContext == context ? null : variant);
                    if (translations.TryGetValue(key, out var values) && !string.IsNullOrEmpty(values[0]))
                    {
                        return values[0];
                    }
                }
            }
            return null;
        }

        internal ReciteManagedPluralResolution ResolvePlural(string id, string sourceSingular, string sourcePlural, long count, ReciteLocaleTextDomain domain, string locale, string variant)
        {
            var context = Context(id, domain);
            var attempts = new List<ReciteManagedPluralAttempt>();
            foreach (var candidateContext in Contexts(context, variant))
            {
                foreach (var candidateLocale in LocaleFallbacks(locale))
                {
                    if (!pluralRules.TryGetValue(candidateLocale, out var rule))
                    {
                        attempts.Add(new ReciteManagedPluralAttempt(candidateLocale, candidateContext, id, null, "missing_plural_forms"));
                        continue;
                    }

                    var arm = ReciteNativeBridge.EvaluatePluralRule(rule.Header, count, rule.ArmCount);
                    var key = CatalogKey.Plural(candidateLocale, id, sourceSingular, sourcePlural, candidateContext == context ? null : variant);
                    if (!translations.TryGetValue(key, out var values))
                    {
                        attempts.Add(new ReciteManagedPluralAttempt(candidateLocale, candidateContext, id, arm, "missing_entry"));
                        continue;
                    }
                    if (arm >= values.Length || string.IsNullOrEmpty(values[arm]))
                    {
                        attempts.Add(new ReciteManagedPluralAttempt(candidateLocale, candidateContext, id, arm, "missing_translation"));
                        continue;
                    }

                    attempts.Add(new ReciteManagedPluralAttempt(candidateLocale, candidateContext, id, arm, "matched"));
                    return new ReciteManagedPluralResolution(values[arm], arm, candidateLocale, candidateContext, id, attempts);
                }
            }
            return new ReciteManagedPluralResolution(null, null, null, null, null, attempts);
        }

        private void AddEntry(CatalogKey key, string[] value)
        {
            if (translations.TryGetValue(key, out var existing) && !ArraysEqual(existing, value))
            {
                throw new ArgumentException("conflicting locale catalogue entry", nameof(key));
            }
            translations[key] = value;
        }

        private static bool ArraysEqual(string[] left, string[] right)
        {
            if (left.Length != right.Length) return false;
            for (var index = 0; index < left.Length; index++)
            {
                if (!StringComparer.Ordinal.Equals(left[index], right[index])) return false;
            }
            return true;
        }

        private static string ValidateLocale(string value)
        {
            return ReciteStringValidation.ValidateLocale(value, nameof(value));
        }

        private static string ValidateString(string value, string parameterName)
        {
            return ReciteStringValidation.Validate(value, parameterName, allowEmpty: true);
        }

        private static void ValidateTranslation(string source, string translation)
        {
            if (!string.IsNullOrEmpty(translation))
            {
                ReciteNativeBridge.ValidateTranslationPlaceholders(source, translation);
            }
        }

        private static void RequireText(string value, string parameterName)
        {
            ReciteStringValidation.Validate(value, parameterName);
        }

        private static string Context(string id, ReciteLocaleTextDomain domain)
        {
            switch (domain)
            {
                case ReciteLocaleTextDomain.AvailabilityReason: return "availability_reason:" + id;
                case ReciteLocaleTextDomain.PresentationLabel: return "presentation_label:" + id;
                default: return id;
            }
        }

        private static IEnumerable<string> Contexts(string context, string variant)
        {
            if (!string.IsNullOrEmpty(variant)) yield return context + "&" + variant;
            yield return context;
        }

        private static IEnumerable<string> LocaleFallbacks(string locale)
        {
            var current = locale;
            while (!string.IsNullOrEmpty(current))
            {
                yield return current;
                var separator = current.LastIndexOf('-');
                if (separator <= 0) yield break;
                current = current.Substring(0, separator);
            }
        }

        private sealed class PluralRule
        {
            private readonly int armCount;
            private readonly string header;

            internal PluralRule(int armCount, string header)
            {
                this.armCount = armCount;
                this.header = header;
            }

            internal int ArmCount => armCount;

            internal string Header => header;

        }

        private sealed class CatalogKey
        {
            internal readonly string Locale;
            internal readonly ReciteLocaleTextDomain Domain;
            internal readonly string Context;
            internal readonly string SourceText;
            internal readonly string PluralSourceText;

            private CatalogKey(string locale, ReciteLocaleTextDomain domain, string context, string sourceText, string pluralSourceText)
            {
                Locale = locale; Domain = domain; Context = context; SourceText = sourceText; PluralSourceText = pluralSourceText;
            }

            internal static CatalogKey Singular(string locale, ReciteLocaleTextDomain domain, string id, string sourceText, string variant)
            {
                return new CatalogKey(locale, domain, Context(id, domain) + (string.IsNullOrEmpty(variant) ? string.Empty : "&" + variant), sourceText, null);
            }

            internal static CatalogKey Plural(string locale, string id, string sourceSingular, string sourcePlural, string variant)
            {
                return new CatalogKey(locale, ReciteLocaleTextDomain.Line, Context(id, ReciteLocaleTextDomain.Line) + (string.IsNullOrEmpty(variant) ? string.Empty : "&" + variant), sourceSingular, sourcePlural);
            }
        }

        private sealed class CatalogKeyComparer : IEqualityComparer<CatalogKey>
        {
            internal static readonly CatalogKeyComparer Instance = new CatalogKeyComparer();

            public bool Equals(CatalogKey left, CatalogKey right)
            {
                return left != null && right != null && left.Domain == right.Domain
                    && StringComparer.Ordinal.Equals(left.Locale, right.Locale)
                    && StringComparer.Ordinal.Equals(left.Context, right.Context)
                    && StringComparer.Ordinal.Equals(left.SourceText, right.SourceText)
                    && StringComparer.Ordinal.Equals(left.PluralSourceText, right.PluralSourceText);
            }

            public int GetHashCode(CatalogKey value)
            {
                unchecked
                {
                    var hash = 17;
                    hash = hash * 31 + StringComparer.Ordinal.GetHashCode(value.Locale);
                    hash = hash * 31 + (int)value.Domain;
                    hash = hash * 31 + StringComparer.Ordinal.GetHashCode(value.Context);
                    hash = hash * 31 + StringComparer.Ordinal.GetHashCode(value.SourceText);
                    hash = hash * 31 + (value.PluralSourceText == null ? 0 : StringComparer.Ordinal.GetHashCode(value.PluralSourceText));
                    return hash;
                }
            }
        }
    }

    internal sealed class ReciteManagedPluralResolution
    {
        internal ReciteManagedPluralResolution(string text, int? selectedArm, string locale, string context, string key, IReadOnlyList<ReciteManagedPluralAttempt> attempts)
        {
            Text = text; SelectedArm = selectedArm; MatchedLocale = locale; MatchedContext = context; MatchedKey = key; Attempts = attempts;
        }

        internal string Text { get; }
        internal int? SelectedArm { get; }
        internal string MatchedLocale { get; }
        internal string MatchedContext { get; }
        internal string MatchedKey { get; }
        internal IReadOnlyList<ReciteManagedPluralAttempt> Attempts { get; }
    }

    internal sealed class ReciteManagedPluralAttempt
    {
        internal ReciteManagedPluralAttempt(string locale, string context, string key, int? selectedArm, string outcome)
        {
            Locale = locale; Context = context; Key = key; SelectedArm = selectedArm; Outcome = outcome;
        }

        internal string Locale { get; }
        internal string Context { get; }
        internal string Key { get; }
        internal int? SelectedArm { get; }
        internal string Outcome { get; }
    }
}
