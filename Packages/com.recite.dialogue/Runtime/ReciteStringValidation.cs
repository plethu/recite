using System;

namespace Recite.Unity
{
    internal static class ReciteStringValidation
    {
        internal static string Validate(string value, string parameterName, bool allowNull = false, bool allowEmpty = false)
        {
            if (value == null)
            {
                if (allowNull) return null;
                throw new ArgumentNullException(parameterName);
            }
            if (!allowEmpty && value.Length == 0)
            {
                throw new ArgumentException("value is required", parameterName);
            }

            for (var index = 0; index < value.Length; index++)
            {
                var character = value[index];
                if (character == '\0')
                {
                    throw new ArgumentException("strings cannot contain embedded NUL characters", parameterName);
                }
                if (char.IsHighSurrogate(character))
                {
                    if (index + 1 >= value.Length || !char.IsLowSurrogate(value[index + 1]))
                    {
                        throw new ArgumentException("strings cannot contain unpaired UTF-16 surrogates", parameterName);
                    }
                    index++;
                }
                else if (char.IsLowSurrogate(character))
                {
                    throw new ArgumentException("strings cannot contain unpaired UTF-16 surrogates", parameterName);
                }
            }
            return value;
        }

        internal static string ValidateLocale(string value, string parameterName)
        {
            var validated = Validate(value, parameterName);
            if (validated.Trim().Length == 0)
            {
                throw new ArgumentException("locale is required", parameterName);
            }

            return validated;
        }
    }
}
