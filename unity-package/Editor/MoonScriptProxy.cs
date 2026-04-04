using System;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEditor;
using UnityEditor.Callbacks;

namespace Moon.Editor
{
    /// <summary>
    /// Enables Moon scripts to work with:
    /// - AddComponent menu (searches generated MonoScripts)
    /// - Drag-and-drop .mn files onto GameObjects (adds the generated component)
    /// - Double-click .mn files (opens in external editor)
    /// </summary>
    [InitializeOnLoad]
    public static class MoonScriptProxy
    {
        static MoonScriptProxy()
        {
            // Register drag-and-drop handler
            DragAndDrop.AddDropHandlerV2(OnHierarchyDrop);
            DragAndDrop.AddDropHandlerV2(OnInspectorDrop);
        }

        /// <summary>
        /// Handle drag-and-drop of .mn files onto Hierarchy.
        /// </summary>
        private static DragAndDropVisualMode OnHierarchyDrop(
            EntityId dropTargetInstanceID,
            HierarchyDropFlags dropMode,
            Transform parentForDraggedObjects,
            bool perform)
        {
            return HandleDrop(dropTargetInstanceID, perform);
        }

        /// <summary>
        /// Handle drag-and-drop of .mn files onto Inspector.
        /// </summary>
        private static DragAndDropVisualMode OnInspectorDrop(
            UnityEngine.Object[] targets,
            bool perform)
        {
            if (targets == null || targets.Length == 0)
                return DragAndDropVisualMode.None;

            var go = targets[0] as GameObject;
            if (go == null)
            {
                var comp = targets[0] as Component;
                if (comp != null) go = comp.gameObject;
            }
            if (go == null) return DragAndDropVisualMode.None;

            return HandleDropOnGameObject(go, perform);
        }

        private static DragAndDropVisualMode HandleDrop(EntityId targetEntityId, bool perform)
        {
            var mnAssets = DragAndDrop.objectReferences
                .Where(o => o != null && AssetDatabase.GetAssetPath(o).EndsWith(".mn"))
                .ToArray();

            if (mnAssets.Length == 0)
                return DragAndDropVisualMode.None;

            if (!perform)
                return DragAndDropVisualMode.Link;

            var targetObj = EditorUtility.EntityIdToObject(targetEntityId) as GameObject;
            if (targetObj == null)
                return DragAndDropVisualMode.None;

            foreach (var mnAsset in mnAssets)
            {
                AddMoonComponent(targetObj, AssetDatabase.GetAssetPath(mnAsset));
            }

            return DragAndDropVisualMode.Link;
        }

        private static DragAndDropVisualMode HandleDropOnGameObject(GameObject go, bool perform)
        {
            var mnAssets = DragAndDrop.objectReferences
                .Where(o => o != null && AssetDatabase.GetAssetPath(o).EndsWith(".mn"))
                .ToArray();

            if (mnAssets.Length == 0)
                return DragAndDropVisualMode.None;

            if (!perform)
                return DragAndDropVisualMode.Link;

            foreach (var mnAsset in mnAssets)
            {
                AddMoonComponent(go, AssetDatabase.GetAssetPath(mnAsset));
            }

            return DragAndDropVisualMode.Link;
        }

        /// <summary>
        /// Add the generated MonoBehaviour component from a .mn file to a GameObject.
        /// </summary>
        public static bool AddMoonComponent(GameObject go, string mnAssetPath)
        {
            string className = Path.GetFileNameWithoutExtension(mnAssetPath);
            MonoScript script = FindGeneratedScript(className);

            if (script == null)
            {
                Debug.LogWarning($"[Moon] Generated script not found for '{className}'. Build the project first (Moon > Build Project).");
                return false;
            }

            Type scriptType = script.GetClass();
            if (scriptType == null)
            {
                Debug.LogWarning($"[Moon] Type '{className}' not found. The generated C# may have compilation errors.");
                return false;
            }

            if (!typeof(MonoBehaviour).IsAssignableFrom(scriptType))
            {
                Debug.LogWarning($"[Moon] '{className}' is not a MonoBehaviour and cannot be added as a component.");
                return false;
            }

            Undo.AddComponent(go, scriptType);
            Debug.Log($"[Moon] Added component '{className}' to {go.name}");
            return true;
        }

        /// <summary>
        /// Find the generated MonoScript by class name.
        /// Searches in the generated package output directory.
        /// </summary>
        public static MonoScript FindGeneratedScript(string className)
        {
            // Search all MonoScripts for matching class name
            string[] guids = AssetDatabase.FindAssets($"t:MonoScript {className}");
            foreach (string guid in guids)
            {
                string path = AssetDatabase.GUIDToAssetPath(guid);
                MonoScript script = AssetDatabase.LoadAssetAtPath<MonoScript>(path);
                if (script != null && script.name == className)
                {
                    return script;
                }
            }

            return null;
        }

        /// <summary>
        /// Intercept asset open:
        /// - .mn file → open in VSCode
        /// - Generated .cs file → redirect to corresponding .mn file
        /// - Inspector script field double-click → redirect to .mn if exists
        /// </summary>
        [OnOpenAsset(0)] // Priority 0 = runs before default handler
        #pragma warning disable CS0618 // OnOpenAsset requires int instanceID
        public static bool OnOpenMoonAsset(int instanceID, int line)
        {
            var obj = EditorUtility.InstanceIDToObject(instanceID);
        #pragma warning restore CS0618
            if (obj == null) return false;
            string path = AssetDatabase.GetAssetPath(obj);

            // Case 1: Direct .mn file double-click
            if (path.EndsWith(".mn"))
            {
                OpenInEditor(Path.Combine(MoonProjectSettings.GetProjectRoot(), path), line);
                return true;
            }

            // Case 2: Generated .cs file — redirect to .mn source
            if (path.EndsWith(".cs"))
            {
                string outputDir = MoonProjectSettings.GetOutputDir();
                if (path.StartsWith(outputDir) || path.Contains("com.moon.generated"))
                {
                    string className = Path.GetFileNameWithoutExtension(path);
                    string mnPath = FindMoonSource(className);
                    if (mnPath != null)
                    {
                        OpenInEditor(Path.Combine(MoonProjectSettings.GetProjectRoot(), mnPath), line);
                        return true;
                    }
                }
            }

            return false; // Let Unity handle it
        }

        /// <summary>
        /// Find the .mn source file for a given class name.
        /// </summary>
        private static string FindMoonSource(string className)
        {
            string[] guids = AssetDatabase.FindAssets(className + " t:TextAsset");
            foreach (string guid in guids)
            {
                string p = AssetDatabase.GUIDToAssetPath(guid);
                if (p.EndsWith(".mn") && Path.GetFileNameWithoutExtension(p) == className)
                    return p;
            }

            // Brute search in Assets
            string[] mnFiles = AssetDatabase.FindAssets("t:DefaultAsset");
            foreach (string guid in mnFiles)
            {
                string p = AssetDatabase.GUIDToAssetPath(guid);
                if (p.EndsWith(".mn") && Path.GetFileNameWithoutExtension(p) == className)
                    return p;
            }

            return null;
        }

        private static void OpenInEditor(string fullPath, int line)
        {
            try
            {
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = "code",
                    Arguments = $"--goto \"{fullPath}\":{Math.Max(1, line)}",
                    UseShellExecute = true,
                    CreateNoWindow = true
                };
                System.Diagnostics.Process.Start(psi);
            }
            catch
            {
                EditorUtility.OpenWithDefaultApp(fullPath);
            }
        }
    }
}
