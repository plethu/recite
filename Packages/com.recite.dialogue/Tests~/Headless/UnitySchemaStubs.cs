using System;
namespace UnityEngine
{
    public class ScriptableObject { }
    [AttributeUsage(AttributeTargets.Class)]
    public sealed class CreateAssetMenuAttribute : Attribute
    {
        public string menuName;
        public string fileName;
    }
    [AttributeUsage(AttributeTargets.Field)]
    public sealed class TooltipAttribute : Attribute
    {
        public TooltipAttribute(string text) { }
    }
}
