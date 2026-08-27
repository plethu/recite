using System.Collections.Generic;
using System.Text;

namespace Recite.Unity.Native
{
    internal sealed class ReciteMessagePackWriter
    {
        private readonly List<byte> bytes = new List<byte>();

        internal byte[] ToArray()
        {
            return bytes.ToArray();
        }

        internal void WriteMapHeader(int len)
        {
            if (len < 16)
            {
                bytes.Add((byte)(0x80 | len));
                return;
            }

            bytes.Add(0xde);
            WriteUInt16((ushort)len);
        }

        internal void WriteArrayHeader(int len)
        {
            if (len < 16)
            {
                bytes.Add((byte)(0x90 | len));
                return;
            }

            bytes.Add(0xdc);
            WriteUInt16((ushort)len);
        }

        internal void WriteString(string value)
        {
            var data = Encoding.UTF8.GetBytes(value ?? string.Empty);
            if (data.Length < 32)
            {
                bytes.Add((byte)(0xa0 | data.Length));
            }
            else if (data.Length <= byte.MaxValue)
            {
                bytes.Add(0xd9);
                bytes.Add((byte)data.Length);
            }
            else if (data.Length <= ushort.MaxValue)
            {
                bytes.Add(0xda);
                WriteUInt16((ushort)data.Length);
            }
            else
            {
                bytes.Add(0xdb);
                WriteUInt32((uint)data.Length);
            }

            bytes.AddRange(data);
        }

        internal void WriteBool(bool value)
        {
            bytes.Add(value ? (byte)0xc3 : (byte)0xc2);
        }

        internal void WriteNull()
        {
            bytes.Add(0xc0);
        }

        internal void WriteRaw(byte[] value)
        {
            bytes.AddRange(value ?? new byte[0]);
        }

        private void WriteUInt16(ushort value)
        {
            bytes.Add((byte)(value >> 8));
            bytes.Add((byte)value);
        }

        private void WriteUInt32(uint value)
        {
            bytes.Add((byte)(value >> 24));
            bytes.Add((byte)(value >> 16));
            bytes.Add((byte)(value >> 8));
            bytes.Add((byte)value);
        }
    }
}
