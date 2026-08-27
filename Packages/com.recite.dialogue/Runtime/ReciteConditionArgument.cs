using System;

namespace Recite.Unity
{
    public enum ReciteConditionArgumentKind
    {
        Identifier,
        String,
        Integer,
        Float,
        Boolean
    }

    public sealed class ReciteConditionArgument
    {
        private ReciteConditionArgument(
            ReciteConditionArgumentKind kind,
            string identifierValue,
            string stringValue,
            long integerValue,
            double floatValue,
            bool booleanValue)
        {
            Kind = kind;
            IdentifierValue = identifierValue;
            StringValue = stringValue;
            IntegerValue = integerValue;
            FloatValue = floatValue;
            BooleanValue = booleanValue;
        }

        public ReciteConditionArgumentKind Kind { get; }

        public string IdentifierValue { get; }

        public string StringValue { get; }

        public long IntegerValue { get; }

        public double FloatValue { get; }

        public bool BooleanValue { get; }

        public static ReciteConditionArgument Identifier(string value)
        {
            return new ReciteConditionArgument(
                ReciteConditionArgumentKind.Identifier,
                value ?? string.Empty,
                null,
                0,
                0,
                false);
        }

        public static ReciteConditionArgument String(string value)
        {
            return new ReciteConditionArgument(
                ReciteConditionArgumentKind.String,
                null,
                value ?? string.Empty,
                0,
                0,
                false);
        }

        public static ReciteConditionArgument Integer(long value)
        {
            return new ReciteConditionArgument(
                ReciteConditionArgumentKind.Integer,
                null,
                null,
                value,
                0,
                false);
        }

        public static ReciteConditionArgument Float(double value)
        {
            if (double.IsNaN(value) || double.IsInfinity(value))
            {
                throw new ArgumentOutOfRangeException(nameof(value), "condition float must be finite");
            }

            return new ReciteConditionArgument(
                ReciteConditionArgumentKind.Float,
                null,
                null,
                0,
                value,
                false);
        }

        public static ReciteConditionArgument Boolean(bool value)
        {
            return new ReciteConditionArgument(
                ReciteConditionArgumentKind.Boolean,
                null,
                null,
                0,
                0,
                value);
        }

        internal object LegacyValue
        {
            get
            {
                switch (Kind)
                {
                    case ReciteConditionArgumentKind.Identifier:
                        return IdentifierValue;
                    case ReciteConditionArgumentKind.String:
                        return StringValue;
                    case ReciteConditionArgumentKind.Integer:
                        return IntegerValue;
                    case ReciteConditionArgumentKind.Float:
                        return FloatValue;
                    case ReciteConditionArgumentKind.Boolean:
                        return BooleanValue;
                    default:
                        throw new InvalidOperationException("unknown Recite condition argument kind");
                }
            }
        }
    }
}
