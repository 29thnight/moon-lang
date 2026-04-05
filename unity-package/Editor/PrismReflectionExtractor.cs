using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using UnityEngine;
using UnityEditor;

namespace Prism.Editor
{
    /// <summary>
    /// Extracts type metadata from Unity assemblies via reflection.
    /// Outputs JSON for VSCode extension to consume.
    /// Captures: Obsolete attributes, exact signatures, generic constraints, serialization info.
    /// </summary>
    public static class PrismReflectionExtractor
    {
        private const string OutputDir = "Library/PrSM";
        private const string OutputFile = "reflection_data.json";

        [MenuItem("PrSM/Extract API Metadata", priority = 200)]
        public static void ExtractAll()
        {
            EditorUtility.DisplayProgressBar("PrSM", "Extracting API metadata...", 0f);

            try
            {
                var data = new ReflectionData();

                // Target assemblies
                string[] assemblyNames = {
                    "UnityEngine.CoreModule",
                    "UnityEngine.PhysicsModule",
                    "UnityEngine.AnimationModule",
                    "UnityEngine.AudioModule",
                    "UnityEngine.UIModule",
                    "UnityEngine.UI",
                    "UnityEngine.IMGUIModule",
                    "UnityEngine.ParticleSystemModule",
                    "UnityEngine.AIModule",
                    "UnityEngine.InputLegacyModule",
                    "Unity.TextMeshPro",
                };

                var allAssemblies = AppDomain.CurrentDomain.GetAssemblies();
                int processed = 0;

                foreach (string asmName in assemblyNames)
                {
                    var asm = allAssemblies.FirstOrDefault(a => a.GetName().Name == asmName);
                    if (asm == null) continue;

                    EditorUtility.DisplayProgressBar("PrSM",
                        $"Scanning {asmName}...",
                        (float)processed / assemblyNames.Length);

                    foreach (var type in asm.GetExportedTypes())
                    {
                        if (type.IsNotPublic) continue;

                        var typeInfo = ExtractType(type);
                        if (typeInfo != null)
                        {
                            data.types.Add(typeInfo);
                        }
                    }

                    processed++;
                }

                // Write output
                Directory.CreateDirectory(OutputDir);
                string outputPath = Path.Combine(OutputDir, OutputFile);
                string json = JsonUtility.ToJson(data, true);
                File.WriteAllText(outputPath, json);

                Debug.Log($"[PrSM] Extracted {data.types.Count} types to {outputPath}");
            }
            finally
            {
                EditorUtility.ClearProgressBar();
            }
        }

        private static TypeInfo ExtractType(Type type)
        {
            var info = new TypeInfo
            {
                name = type.Name,
                fullName = type.FullName,
                @namespace = type.Namespace ?? "",
                isObsolete = HasObsolete(type),
                obsoleteMessage = GetObsoleteMessage(type),
                isSerializable = type.IsSerializable || type.GetCustomAttribute<SerializableAttribute>() != null,
                isAbstract = type.IsAbstract,
                isSealed = type.IsSealed,
                isInterface = type.IsInterface,
                isEnum = type.IsEnum,
                isStruct = type.IsValueType && !type.IsEnum,
                baseType = type.BaseType?.Name ?? "",
            };

            // Members
            var flags = BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;

            // Properties
            foreach (var prop in type.GetProperties(flags))
            {
                info.members.Add(new MemberInfo
                {
                    name = prop.Name,
                    kind = "Property",
                    returnType = FormatType(prop.PropertyType),
                    signature = FormatPropertySignature(prop),
                    isStatic = prop.GetGetMethod()?.IsStatic ?? false,
                    isObsolete = HasObsolete(prop),
                    obsoleteMessage = GetObsoleteMessage(prop),
                });
            }

            // Methods
            foreach (var method in type.GetMethods(flags))
            {
                if (method.IsSpecialName) continue; // skip property accessors, operators

                info.members.Add(new MemberInfo
                {
                    name = method.Name,
                    kind = "Method",
                    returnType = FormatType(method.ReturnType),
                    signature = FormatMethodSignature(method),
                    isStatic = method.IsStatic,
                    isObsolete = HasObsolete(method),
                    obsoleteMessage = GetObsoleteMessage(method),
                });
            }

            // Fields (public only)
            foreach (var field in type.GetFields(flags))
            {
                info.members.Add(new MemberInfo
                {
                    name = field.Name,
                    kind = "Field",
                    returnType = FormatType(field.FieldType),
                    signature = $"public {FormatType(field.FieldType)} {field.Name}",
                    isStatic = field.IsStatic,
                    isObsolete = HasObsolete(field),
                    obsoleteMessage = GetObsoleteMessage(field),
                });
            }

            // Events
            foreach (var evt in type.GetEvents(flags))
            {
                info.members.Add(new MemberInfo
                {
                    name = evt.Name,
                    kind = "Event",
                    returnType = FormatType(evt.EventHandlerType),
                    signature = $"event {FormatType(evt.EventHandlerType)} {evt.Name}",
                    isStatic = false,
                    isObsolete = HasObsolete(evt),
                    obsoleteMessage = GetObsoleteMessage(evt),
                });
            }

            // Enum values
            if (type.IsEnum)
            {
                foreach (string enumName in Enum.GetNames(type))
                {
                    info.members.Add(new MemberInfo
                    {
                        name = enumName,
                        kind = "EnumValue",
                        returnType = type.Name,
                        signature = enumName,
                        isStatic = true,
                        isObsolete = false,
                        obsoleteMessage = "",
                    });
                }
            }

            return info;
        }

        // ── Helpers ──────────────────────────────────

        private static bool HasObsolete(System.Reflection.MemberInfo member)
        {
            return member.GetCustomAttribute<ObsoleteAttribute>() != null;
        }

        private static string GetObsoleteMessage(System.Reflection.MemberInfo member)
        {
            var attr = member.GetCustomAttribute<ObsoleteAttribute>();
            return attr?.Message ?? "";
        }

        private static string FormatMethodSignature(MethodInfo method)
        {
            var @params = method.GetParameters()
                .Select(p => $"{FormatType(p.ParameterType)} {p.Name}")
                .ToArray();

            string modifiers = method.IsStatic ? "public static" : "public";
            return $"{modifiers} {FormatType(method.ReturnType)} {method.Name}({string.Join(", ", @params)})";
        }

        private static string FormatPropertySignature(PropertyInfo prop)
        {
            string modifiers = (prop.GetGetMethod()?.IsStatic ?? false) ? "public static" : "public";
            string accessors = "";
            if (prop.CanRead && prop.CanWrite) accessors = "{ get; set; }";
            else if (prop.CanRead) accessors = "{ get; }";
            else if (prop.CanWrite) accessors = "{ set; }";
            return $"{modifiers} {FormatType(prop.PropertyType)} {prop.Name} {accessors}";
        }

        private static string FormatType(Type type)
        {
            if (type == null) return "void";
            if (type == typeof(void)) return "void";
            if (type == typeof(int)) return "int";
            if (type == typeof(float)) return "float";
            if (type == typeof(double)) return "double";
            if (type == typeof(bool)) return "bool";
            if (type == typeof(string)) return "string";
            if (type == typeof(char)) return "char";
            if (type == typeof(long)) return "long";
            if (type == typeof(byte)) return "byte";

            if (type.IsGenericType)
            {
                string name = type.Name.Split('`')[0];
                string args = string.Join(", ", type.GetGenericArguments().Select(FormatType));
                return $"{name}<{args}>";
            }

            if (type.IsArray)
            {
                return $"{FormatType(type.GetElementType())}[]";
            }

            return type.Name;
        }
    }

    // ── Data structures for JSON serialization ───────

    [Serializable]
    public class ReflectionData
    {
        public List<TypeInfo> types = new List<TypeInfo>();
    }

    [Serializable]
    public class TypeInfo
    {
        public string name;
        public string fullName;
        public string @namespace;
        public string baseType;
        public bool isObsolete;
        public string obsoleteMessage;
        public bool isSerializable;
        public bool isAbstract;
        public bool isSealed;
        public bool isInterface;
        public bool isEnum;
        public bool isStruct;
        public List<MemberInfo> members = new List<MemberInfo>();
    }

    [Serializable]
    public class MemberInfo
    {
        public string name;
        public string kind;
        public string returnType;
        public string signature;
        public bool isStatic;
        public bool isObsolete;
        public string obsoleteMessage;
    }
}
