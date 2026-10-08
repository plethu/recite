using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
#if UNITY_2022_3_OR_NEWER
using AOT;
#endif
using Recite.Unity.Native;

namespace Recite.Unity
{
    // Owns condition registration and the host-owned callback borrows.
    internal sealed class ConditionCallbacks : IDisposable
    {
        private readonly Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>> conditions = new Dictionary<string, Func<IReadOnlyList<object>, ReciteConditionValue>>(StringComparer.Ordinal);
        private readonly Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>> typedConditions = new Dictionary<string, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue>>(StringComparer.Ordinal);
        private static readonly ReciteNativeBridge.ReciteConditionFn conditionCallback = ConditionCallbackEntry;
        private GCHandle pinnedConditionValue;
        private GCHandle pinnedConditionError;
        private readonly GCHandle contextHandle;
        private readonly IntPtr contextPointer;

        internal ConditionCallbacks()
        {
            contextHandle = GCHandle.Alloc(this, GCHandleType.Normal);
            contextPointer = GCHandle.ToIntPtr(contextHandle);
        }

        internal void RegisterCondition(string name, Func<IReadOnlyList<object>, bool> handler)
        {
            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            RegisterConditionValue(name, args => ReciteConditionValue.Bool(handler(args)));
        }

        internal void RegisterConditionValue(string name, Func<IReadOnlyList<object>, ReciteConditionValue> handler)
        {
            ReciteStringValidation.Validate(name, nameof(name));
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("condition name is required", nameof(name));
            }

            conditions[name] = handler ?? throw new ArgumentNullException(nameof(handler));
            typedConditions.Remove(name);
        }

        internal void RegisterTypedCondition(string name, Func<IReadOnlyList<ReciteConditionArgument>, bool> handler)
        {
            if (handler == null)
            {
                throw new ArgumentNullException(nameof(handler));
            }

            RegisterTypedConditionValue(name, args => ReciteConditionValue.Bool(handler(args)));
        }

        internal void RegisterTypedConditionValue(string name, Func<IReadOnlyList<ReciteConditionArgument>, ReciteConditionValue> handler)
        {
            ReciteStringValidation.Validate(name, nameof(name));
            if (string.IsNullOrWhiteSpace(name))
            {
                throw new ArgumentException("condition name is required", nameof(name));
            }

            typedConditions[name] = handler ?? throw new ArgumentNullException(nameof(handler));
            conditions.Remove(name);
        }

        internal void Register(ulong sessionHandle)
        {
            var names = new HashSet<string>(conditions.Keys, StringComparer.Ordinal);
            names.UnionWith(typedConditions.Keys);
            var registeredNames = new List<string>(names);
            registeredNames.Sort(StringComparer.Ordinal);
            foreach (var name in registeredNames)
            {
                ReciteStringValidation.Validate(name, nameof(name));
                ReciteNativeBridge.ThrowIfError(ReciteNativeBridge.SessionRegisterCondition(sessionHandle, ReciteNativeBridge.ToUtf8NullTerminated(name), conditionCallback, contextPointer));
            }
        }

        // Internal so the managed headless fixture can exercise the same callback
        // boundary used by the native bridge without requiring a Unity binary.
        internal ReciteNativeBridge.ReciteConditionResult EvaluateCondition(IntPtr queryPtr, IntPtr userdata)
        {
            return EvaluateConditionCore(queryPtr);
        }

#if UNITY_2022_3_OR_NEWER
        [MonoPInvokeCallback(typeof(ReciteNativeBridge.ReciteConditionFn))]
#endif
        private static ReciteNativeBridge.ReciteConditionResult ConditionCallbackEntry(IntPtr queryPtr, IntPtr userdata)
        {
            ConditionCallbacks service = null;
            try
            {
                service = FromContext(userdata);
                return service == null
                    ? InvalidConditionCallbackResult()
                    : service.EvaluateConditionCore(queryPtr);
            }
            catch (Exception error)
            {
                try
                {
                    return service?.ConditionFailure(error.Message) ?? InvalidConditionCallbackResult();
                }
                catch
                {
                    // No managed exception may cross the reverse P/Invoke boundary.
                    return InvalidConditionCallbackResult();
                }
            }
        }

        private ReciteNativeBridge.ReciteConditionResult EvaluateConditionCore(IntPtr queryPtr)
        {
            if (queryPtr == IntPtr.Zero)
            {
                return ConditionFailure("condition query pointer was null");
            }

            var query = Marshal.PtrToStructure<ReciteNativeBridge.ReciteConditionQuery>(queryPtr);
            var name = Marshal.PtrToStringUTF8(query.FunctionName) ?? string.Empty;
            conditions.TryGetValue(name, out var handler);
            typedConditions.TryGetValue(name, out var typedHandler);
            if (handler == null && typedHandler == null)
            {
                return ConditionFailure("no Unity condition handler registered for `" + name + "`");
            }

            try
            {
                var value = typedHandler != null
                    ? typedHandler(ReciteNativeBridge.ReadTypedConditionArgs(query.ArgsMsgpack, query.ArgsLen))
                    : handler(ReciteNativeBridge.ReadConditionArgs(query.ArgsMsgpack, query.ArgsLen));
                if (value == null)
                    return ConditionSuccess(new byte[] { 0xc0 }); // MessagePack nil: native classifies invalid result.
                var encoded = value.IsEnum
                    ? ReciteMessagePack.EncodeConditionEnum(value.EnumVariant)
                    : ReciteMessagePack.EncodeConditionBool(value.BoolValue);
                return ConditionSuccess(encoded);
            }
            catch (Exception ex)
            {
                return ConditionFailure(ex.Message);
            }
        }

        private ReciteNativeBridge.ReciteConditionResult ConditionSuccess(byte[] bytes)
        {
            if (pinnedConditionValue.IsAllocated)
            {
                pinnedConditionValue.Free();
            }

            pinnedConditionValue = GCHandle.Alloc(bytes, GCHandleType.Pinned);
            return new ReciteNativeBridge.ReciteConditionResult
            {
                Ok = 1,
                ValueMsgpack = pinnedConditionValue.AddrOfPinnedObject(),
                ValueLen = new UIntPtr((ulong)bytes.Length),
                ErrorMessage = IntPtr.Zero
            };
        }

        private ReciteNativeBridge.ReciteConditionResult ConditionFailure(string message)
        {
            var bytes = ReciteNativeBridge.ToUtf8NullTerminated(message ?? "Unity condition handler failed");
            if (pinnedConditionError.IsAllocated)
            {
                pinnedConditionError.Free();
            }

            pinnedConditionError = GCHandle.Alloc(bytes, GCHandleType.Pinned);
            return new ReciteNativeBridge.ReciteConditionResult
            {
                Ok = 0,
                ValueMsgpack = IntPtr.Zero,
                ValueLen = UIntPtr.Zero,
                ErrorMessage = pinnedConditionError.AddrOfPinnedObject()
            };
        }

        private static ConditionCallbacks FromContext(IntPtr userdata)
        {
            if (userdata == IntPtr.Zero)
            {
                return null;
            }
            return GCHandle.FromIntPtr(userdata).Target as ConditionCallbacks;
        }

        private static ReciteNativeBridge.ReciteConditionResult InvalidConditionCallbackResult()
        {
            return new ReciteNativeBridge.ReciteConditionResult { Ok = 0 };
        }

        internal void ReleaseBorrowedResults()
        {
            if (pinnedConditionValue.IsAllocated) pinnedConditionValue.Free();
            if (pinnedConditionError.IsAllocated) pinnedConditionError.Free();
        }

        public void Dispose()
        {
            ReleaseBorrowedResults();
            if (contextHandle.IsAllocated) contextHandle.Free();
        }
    }
}
