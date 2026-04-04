using System;
using System.IO;
using System.Reflection;
using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Assigns Moon icon to generated MonoScripts for Inspector header display.
    /// Disables scene view gizmo icons via internal AnnotationUtility.
    /// </summary>
    [InitializeOnLoad]
    public static class MoonIconAssigner
    {
        private static Texture2D _moonIcon;

        static MoonIconAssigner()
        {
            // First pass: assign icons
            EditorApplication.delayCall += () =>
            {
                AssignIconsToGeneratedScripts();

                // Second pass (next frame): disable scene gizmos
                // Annotations become available after icon assignment + import
                EditorApplication.delayCall += DisableSceneGizmosForGeneratedScripts;
            };
        }

        public static void AssignIconsToGeneratedScripts()
        {
            string outputDir = MoonProjectSettings.GetOutputDir();
            if (!Directory.Exists(Path.Combine(MoonProjectSettings.GetProjectRoot(), outputDir)))
                return;

            Texture2D icon = GetMoonIcon();
            if (icon == null) return;

            string[] guids = AssetDatabase.FindAssets("t:MonoScript", new[] { outputDir });
            foreach (string guid in guids)
            {
                string path = AssetDatabase.GUIDToAssetPath(guid);
                MonoScript script = AssetDatabase.LoadAssetAtPath<MonoScript>(path);
                if (script == null) continue;

                Texture2D current = EditorGUIUtility.GetIconForObject(script);
                if (current != icon)
                {
                    EditorGUIUtility.SetIconForObject(script, icon);
                    EditorUtility.SetDirty(script);
                }
            }
        }

        public static void AssignIconToScript(string className)
        {
            Texture2D icon = GetMoonIcon();
            if (icon == null) return;

            MonoScript script = MoonScriptProxy.FindGeneratedScript(className);
            if (script == null) return;

            EditorGUIUtility.SetIconForObject(script, icon);
            EditorUtility.SetDirty(script);

            string capturedClassName = className;
            EditorApplication.delayCall += () =>
            {
                MonoScript delayedScript = MoonScriptProxy.FindGeneratedScript(capturedClassName);
                if (delayedScript == null) return;

                DisableSceneGizmoForType(delayedScript, capturedClassName);
            };
        }

        /// <summary>
        /// Uses internal AnnotationUtility to disable scene view icon gizmos
        /// while keeping the Inspector header icon intact.
        /// </summary>
        private static void DisableSceneGizmosForGeneratedScripts()
        {
            string outputDir = MoonProjectSettings.GetOutputDir();
            if (!Directory.Exists(Path.Combine(MoonProjectSettings.GetProjectRoot(), outputDir)))
                return;

            string[] guids = AssetDatabase.FindAssets("t:MonoScript", new[] { outputDir });
            foreach (string guid in guids)
            {
                string path = AssetDatabase.GUIDToAssetPath(guid);
                MonoScript script = AssetDatabase.LoadAssetAtPath<MonoScript>(path);
                if (script == null) continue;

                DisableSceneGizmoForType(script);
            }
        }

        private static void DisableSceneGizmoForType(MonoScript script, string scriptLabel = null)
        {
            try
            {
                if (script == null) return;

                Type classType = script.GetClass();
                if (classType == null) return;

                var editorAsm = typeof(UnityEditor.Editor).Assembly;
                var annotationUtilityType = editorAsm.GetType("UnityEditor.AnnotationUtility");
                if (annotationUtilityType == null) return;

                var getAnnotations = annotationUtilityType.GetMethod("GetAnnotations",
                    BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public);
                var setIconEnabled = annotationUtilityType.GetMethod("SetIconEnabled",
                    BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public);

                if (getAnnotations == null || setIconEnabled == null) return;

                var annotationType = editorAsm.GetType("UnityEditor.Annotation");
                var classIdField = annotationType.GetField("classID",
                    BindingFlags.Public | BindingFlags.Instance);
                var scriptClassField = annotationType.GetField("scriptClass",
                    BindingFlags.Public | BindingFlags.Instance);

                var annotations = (Array)getAnnotations.Invoke(null, null);
                foreach (var annotation in annotations)
                {
                    string scriptClass = (string)scriptClassField.GetValue(annotation);
                    if (scriptClass == classType.Name)
                    {
                        int classId = (int)classIdField.GetValue(annotation);
                        setIconEnabled.Invoke(null, new object[] { classId, scriptClass, 0 });
                        break;
                    }
                }
            }
            catch (Exception e)
            {
                string displayName = !string.IsNullOrWhiteSpace(scriptLabel)
                    ? scriptLabel
                    : (script != null ? script.name : "<destroyed>");
                Debug.LogWarning($"[Moon] Could not disable scene gizmo for {displayName}: {e.Message}");
            }
        }

        public static Texture2D GetMoonIcon()
        {
            if (_moonIcon != null) return _moonIcon;

            string[] searchPaths = {
                "Packages/com.moon.editor/Editor/Icons/moon-script-icon.png",
            };

            foreach (string p in searchPaths)
            {
                _moonIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(p);
                if (_moonIcon != null) return _moonIcon;
            }

            string[] guids = AssetDatabase.FindAssets("moon-script-icon t:Texture2D");
            if (guids.Length > 0)
            {
                string path = AssetDatabase.GUIDToAssetPath(guids[0]);
                _moonIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(path);
            }

            return _moonIcon;
        }
    }
}
