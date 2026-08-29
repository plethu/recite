using System;

namespace Recite.Unity
{
    public enum ReciteInterpolationValueKind : uint
    {
        String,
        Integer,
        Float,
        Boolean
    }

    public sealed class ReciteInterpolationValue
    {
        private ReciteInterpolationValue(
            string name,
            ReciteInterpolationValueKind kind,
            string stringValue,
            long integerValue,
            double floatValue,
            bool booleanValue)
        {
            if (string.IsNullOrEmpty(name))
            {
                throw new ArgumentException("interpolation value name is required", nameof(name));
            }
            RejectEmbeddedNul(name, nameof(name));

            Name = name;
            Kind = kind;
            StringValue = stringValue;
            IntegerValue = integerValue;
            FloatValue = floatValue;
            BooleanValue = booleanValue;
        }

        public string Name { get; }

        public ReciteInterpolationValueKind Kind { get; }

        public string StringValue { get; }

        public long IntegerValue { get; }

        public double FloatValue { get; }

        public bool BooleanValue { get; }

        public static ReciteInterpolationValue String(string name, string value)
        {
            if (value == null)
            {
                throw new ArgumentNullException(nameof(value));
            }
            RejectEmbeddedNul(value, nameof(value));
            return new ReciteInterpolationValue(
                name,
                ReciteInterpolationValueKind.String,
                value,
                0,
                0,
                false);
        }

        public static ReciteInterpolationValue Integer(string name, long value)
        {
            return new ReciteInterpolationValue(name, ReciteInterpolationValueKind.Integer, null, value, 0, false);
        }

        public static ReciteInterpolationValue Float(string name, double value)
        {
            if (double.IsNaN(value) || double.IsInfinity(value))
            {
                throw new ArgumentException("interpolation float must be finite", nameof(value));
            }

            return new ReciteInterpolationValue(name, ReciteInterpolationValueKind.Float, null, 0, value, false);
        }

        public static ReciteInterpolationValue Boolean(string name, bool value)
        {
            return new ReciteInterpolationValue(name, ReciteInterpolationValueKind.Boolean, null, 0, 0, value);
        }

        private static void RejectEmbeddedNul(string value, string parameterName)
        {
            if (value.IndexOf('\0') >= 0)
            {
                throw new ArgumentException(
                    "interpolation value strings cannot contain embedded NUL characters",
                    parameterName);
            }
        }
    }
}
