using System;
using Recite.Unity.Native;

namespace Recite.Unity
{
    // Converts native-owned buffers into typed managed output and classifies bad wire data.
    internal static class OutputDecoder
    {
        internal static ReciteOutputBatch DecodeBatch(ref ReciteNativeBridge.ReciteBuffer batch)
        {
            return DecodeBatchBytes(ReciteNativeBridge.CopyAndFree(ref batch));
        }

        internal static ReciteOutputBatch DecodeBatchBytes(byte[] bytes)
        {
            return Decode(() => ReciteMessagePack.DecodeOutputBatch(bytes));
        }

        internal static ReciteMessagePack.ResumeReportPayload DecodeResumeReport(ref ReciteNativeBridge.ReciteBuffer report)
        {
            var bytes = ReciteNativeBridge.CopyAndFree(ref report);
            return Decode(() => ReciteMessagePack.DecodeResumeReport(bytes));
        }

        private static T Decode<T>(Func<T> decode)
        {
            try
            {
                return decode();
            }
            catch (FormatException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (OverflowException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (InvalidCastException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (System.Collections.Generic.KeyNotFoundException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
            catch (ArgumentException error)
            {
                throw new ReciteAdapterException(ReciteStatus.Validation, error.Message);
            }
        }

    }
}
