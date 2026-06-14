using System;

namespace Recite.Unity
{
    public enum ReciteStatus
    {
        Ok = 0,
        Validation = -1,
        AssetLoadOrDecode = -2,
        StaleOrIncompatible = -3,
        SchemaMismatch = -4,
        NoActiveSession = -5,
        SessionAlreadyActive = -6,
        UnknownStartBlock = -7,
        InvalidChoice = -8,
        UnavailableChoice = -9,
        StaleChoice = -10,
        MissingConditionHandler = -11,
        ConditionEvaluation = -12,
        InvalidConditionResult = -13,
        EffectAcknowledgement = -14,
        RejectedRefresh = -15,
        SaveLoadIncompatibility = -16,
        Localisation = -17,
        MissingProjectionHandler = -18,
        ProjectionEvaluation = -19,
        InvalidProjectionResult = -20,
        InvalidHandle = -21,
        DialogueFault = -22
    }

    public sealed class ReciteAdapterException : Exception
    {
        public ReciteAdapterException(ReciteStatus status, string message)
            : base(string.IsNullOrEmpty(message) ? status.ToString() : message)
        {
            Status = status;
        }

        public ReciteStatus Status { get; }
    }
}
