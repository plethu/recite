using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.Events;

namespace Recite.Unity.GameObjects
{
    public sealed class ReciteDialogueRunner : MonoBehaviour
    {
        private ReciteDialogueService service;
        private readonly Queue<ReciteOutputBatch> pendingOutput = new Queue<ReciteOutputBatch>();
        private bool emitting;

        [SerializeField]
        private ReciteCompiledAsset compiledAsset;

        [SerializeField]
        private string startBlock;

        [SerializeField]
        private string locale;

        [SerializeField]
        private string localeVariant;

        [SerializeField]
        private ReciteOutputEvent output = new ReciteOutputEvent();

        [SerializeField]
        private ReciteErrorEvent error = new ReciteErrorEvent();

        public ReciteDialogueService Service => service ?? throw new InvalidOperationException("ReciteDialogueRunner is not awake yet");
        public ReciteOutputEvent Output => output;
        public ReciteErrorEvent Error => error;

        private void Awake()
        {
            // NativeSession captures the creating thread; Unity field initializers
            // can run on a loading thread during scene deserialization.
            if (service == null) service = new ReciteDialogueService();
        }
        public ReciteCompiledAsset CompiledAsset { get => compiledAsset; set => compiledAsset = value; }

        public void StartDialogue()
        {
            if (compiledAsset == null)
            {
                EmitError(new ReciteAdapterException(ReciteStatus.AssetLoadOrDecode, "compiled Recite asset is not assigned"));
                return;
            }

            try
            {
                var asset = compiledAsset.ToDialogueAsset();
                Emit(Service.Start(asset, string.IsNullOrEmpty(startBlock) ? null : startBlock, string.IsNullOrEmpty(locale) ? null : locale, string.IsNullOrEmpty(localeVariant) ? null : localeVariant));
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
                Emit(Service.SelectChoice(choiceId));
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
                Emit(Service.AcknowledgeEffect(effectRequestId));
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
                Emit(Service.AcknowledgeEffect(effectRequestId, false, failureReason));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        public ReciteSessionSnapshot Snapshot()
        {
            return Service.Snapshot();
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
                var asset = compiledAsset.ToDialogueAsset();
                Emit(Service.Restore(asset, snapshot, string.IsNullOrEmpty(localeVariant) ? null : localeVariant));
            }
            catch (ReciteAdapterException ex)
            {
                EmitError(ex);
            }
        }

        // Unity calls OnDisable on scene exit and before script/domain reload.
        private void OnDisable()
        {
            pendingOutput.Clear();
            service?.End();
        }

        private void OnDestroy()
        {
            pendingOutput.Clear();
            service?.Dispose();
        }

        private void Emit(ReciteOutputBatch batch)
        {
            pendingOutput.Enqueue(batch);
            if (emitting) return;
            emitting = true;
            try
            {
                while (pendingOutput.Count != 0)
                {
                    foreach (var item in pendingOutput.Dequeue().Events)
                        output.Invoke(item);
                }
            }
            finally
            {
                // A throwing UnityEvent listener aborts this delivery. Do not
                // replay stale output during a later operation.
                pendingOutput.Clear();
                emitting = false;
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
