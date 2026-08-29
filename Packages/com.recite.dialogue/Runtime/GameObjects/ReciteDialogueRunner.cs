using System;
using UnityEngine;
using UnityEngine.Events;

namespace Recite.Unity.GameObjects
{
    public sealed class ReciteDialogueRunner : MonoBehaviour
    {
        private readonly ReciteDialogueService service = new ReciteDialogueService();

        [SerializeField]
        private TextAsset compiledAsset;

        [SerializeField]
        private string startBlock;

        [SerializeField]
        private string locale;

        [SerializeField]
        private string localeVariant;

        [SerializeField]
        private ReciteOutputEvent output;

        [SerializeField]
        private ReciteErrorEvent error;

        public ReciteDialogueService Service => service;

        public void StartDialogue()
        {
            if (compiledAsset == null)
            {
                EmitError(new ReciteAdapterException(ReciteStatus.AssetLoadOrDecode, "compiled Recite asset is not assigned"));
                return;
            }

            try
            {
                var asset = new ReciteDialogueAsset(compiledAsset.bytes, compiledAsset.name, compiledAsset.name);
                Emit(service.Start(asset, string.IsNullOrEmpty(startBlock) ? null : startBlock, string.IsNullOrEmpty(locale) ? null : locale, string.IsNullOrEmpty(localeVariant) ? null : localeVariant));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        public void SelectChoice(string choiceId)
        {
            try
            {
                Emit(service.SelectChoice(choiceId));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        public void AcknowledgeEffect(string effectRequestId)
        {
            try
            {
                Emit(service.AcknowledgeEffect(effectRequestId));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        public void FailEffect(string effectRequestId, string failureReason)
        {
            try
            {
                Emit(service.AcknowledgeEffect(effectRequestId, false, failureReason));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        public ReciteSessionSnapshot Snapshot()
        {
            return service.Snapshot();
        }

        public void Restore(ReciteSessionSnapshot snapshot)
        {
            if (compiledAsset == null)
            {
                EmitError(new ReciteAdapterException(ReciteStatus.AssetLoadOrDecode, "compiled Recite asset is not assigned"));
                return;
            }

            try
            {
                var asset = new ReciteDialogueAsset(compiledAsset.bytes, compiledAsset.name, compiledAsset.name);
                Emit(service.Restore(asset, snapshot, string.IsNullOrEmpty(localeVariant) ? null : localeVariant));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        private void OnDestroy()
        {
            service.Dispose();
        }

        private void Emit(ReciteOutputBatch batch)
        {
            foreach (var item in batch.Events)
            {
                output.Invoke(item);
            }
        }

        private void EmitError(ReciteAdapterException exception)
        {
            error.Invoke(exception);
        }

        [Serializable]
        public sealed class ReciteOutputEvent : UnityEvent<ReciteOutput>
        {
        }

        [Serializable]
        public sealed class ReciteErrorEvent : UnityEvent<ReciteAdapterException>
        {
        }
    }
}
