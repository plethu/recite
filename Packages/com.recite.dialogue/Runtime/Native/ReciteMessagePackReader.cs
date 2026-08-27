using System;
using System.Collections.Generic;
using System.Text;

namespace Recite.Unity.Native
{
    internal sealed class ReciteMessagePackReader
    {
        private readonly byte[] bytes;
        private int offset;

        internal ReciteMessagePackReader(byte[] bytes)
        {
            this.bytes = bytes ?? Array.Empty<byte>();
        }

        internal IReadOnlyDictionary<string, object> ReadMap()
        {
            if (!(Read() is IReadOnlyDictionary<string, object> map))
            {
                throw new FormatException("expected a MessagePack map");
            }

            return map;
        }

        internal IReadOnlyList<object> ReadArray()
        {
            if (!(Read() is IReadOnlyList<object> array))
            {
                throw new FormatException("expected a MessagePack array");
            }

            return array;
        }

        internal void EnsureEnd()
        {
            if (offset != bytes.Length)
            {
                throw new FormatException("trailing bytes after MessagePack payload");
            }
        }

        private object Read()
        {
            var marker = ReadByte();
            if (marker <= 0x7f)
            {
                return (long)marker;
            }

            if (marker >= 0xa0 && marker <= 0xbf)
            {
                return ReadString(marker & 0x1f);
            }

            if (marker >= 0x90 && marker <= 0x9f)
            {
                return ReadArray(marker & 0x0f);
            }

            if (marker >= 0x80 && marker <= 0x8f)
            {
                return ReadMap(marker & 0x0f);
            }

            if (marker >= 0xe0)
            {
                return (long)(sbyte)marker;
            }

            switch (marker)
            {
                case 0xc0:
                    return null;
                case 0xc2:
                    return false;
                case 0xc3:
                    return true;
                case 0xca:
                    return ReadSingle();
                case 0xcb:
                    return ReadDouble();
                case 0xcc:
                    return (long)ReadByte();
                case 0xcd:
                    return (long)ReadUInt16();
                case 0xce:
                    return (long)ReadUInt32();
                case 0xcf:
                    var unsignedValue = ReadUInt64();
                    if (unsignedValue > long.MaxValue)
                    {
                        throw new FormatException("MessagePack unsigned integer exceeds Int64");
                    }

                    return (long)unsignedValue;
                case 0xd0:
                    return (long)(sbyte)ReadByte();
                case 0xd1:
                    return (long)(short)ReadUInt16();
                case 0xd2:
                    return (long)(int)ReadUInt32();
                case 0xd3:
                    return unchecked((long)ReadUInt64());
                case 0xd9:
                    return ReadString(ReadByte());
                case 0xda:
                    return ReadString(ReadUInt16());
                case 0xdb:
                    return ReadString(checked((int)ReadUInt32()));
                case 0xdc:
                    return ReadArray(ReadUInt16());
                case 0xdd:
                    return ReadArray(checked((int)ReadUInt32()));
                case 0xde:
                    return ReadMap(ReadUInt16());
                case 0xdf:
                    return ReadMap(checked((int)ReadUInt32()));
                default:
                    throw new FormatException("unsupported MessagePack marker 0x" + marker.ToString("x2"));
            }
        }

        private IReadOnlyList<object> ReadArray(int len)
        {
            var values = new List<object>(len);
            for (var i = 0; i < len; i++)
            {
                values.Add(Read());
            }

            return values;
        }

        private IReadOnlyDictionary<string, object> ReadMap(int len)
        {
            var values = new Dictionary<string, object>(StringComparer.Ordinal);
            for (var i = 0; i < len; i++)
            {
                if (!(Read() is string key))
                {
                    throw new FormatException("MessagePack map keys must be strings");
                }

                if (values.ContainsKey(key))
                {
                    throw new FormatException("duplicate MessagePack map key: " + key);
                }

                values.Add(key, Read());
            }

            return values;
        }

        private string ReadString(int len)
        {
            Ensure(len);
            var text = new UTF8Encoding(false, true).GetString(bytes, offset, len);
            offset += len;
            return text;
        }

        private float ReadSingle()
        {
            return BitConverter.ToSingle(ReadBigEndian(sizeof(float)), 0);
        }

        private double ReadDouble()
        {
            return BitConverter.ToDouble(ReadBigEndian(sizeof(double)), 0);
        }

        private byte[] ReadBigEndian(int len)
        {
            Ensure(len);
            var data = new byte[len];
            Buffer.BlockCopy(bytes, offset, data, 0, len);
            offset += len;
            if (BitConverter.IsLittleEndian)
            {
                Array.Reverse(data);
            }

            return data;
        }

        private byte ReadByte()
        {
            Ensure(1);
            return bytes[offset++];
        }

        private ushort ReadUInt16()
        {
            Ensure(2);
            var value = (ushort)((bytes[offset] << 8) | bytes[offset + 1]);
            offset += 2;
            return value;
        }

        private uint ReadUInt32()
        {
            Ensure(4);
            var value = ((uint)bytes[offset] << 24) | ((uint)bytes[offset + 1] << 16) | ((uint)bytes[offset + 2] << 8) | bytes[offset + 3];
            offset += 4;
            return value;
        }

        private ulong ReadUInt64()
        {
            var hi = (ulong)ReadUInt32();
            var lo = (ulong)ReadUInt32();
            return (hi << 32) | lo;
        }

        private void Ensure(int len)
        {
            if (len < 0 || offset > bytes.Length - len)
            {
                throw new FormatException("truncated MessagePack payload");
            }
        }
    }
}
