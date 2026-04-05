using System;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEditor;
using UnityEditor.Callbacks;

namespace Prism.Editor
{
    /// <summary>
    /// Enables PrSM scripts to work with:
    /// - AddComponent menu (searches generated MonoScripts)
    /// - Drag-and-drop source files onto GameObjects (adds the generated component)
    /// - Double-click source files (opens in external editor)
    /// </summary>
    [InitializeOnLoad]
    public static class PrismScriptProxy
    {
        static PrismScriptProxy()
        {
            // Register drag-and-drop handler
            DragAndDrop.AddDropHandlerV2(OnHierarchyDrop);
            DragAndDrop.AddDropHandlerV2(OnInspectorDrop);
        }

        /// <summary>
        /// Handle drag-and-drop of PrSM source files onto Hierarchy.
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
        /// Handle drag-and-drop of PrSM source files onto Inspector.
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
                .Where(o => o != null && PrismProjectConfig.IsPrismSourceAssetPath(AssetDatabase.GetAssetPath(o)))
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
                AddPrSMComponent(targetObj, AssetDatabase.GetAssetPath(mnAsset));
            }

            return DragAndDropVisualMode.Link;
        }

        private static DragAndDropVisualMode HandleDropOnGameObject(GameObject go, bool perform)
        {
            var mnAssets = DragAndDrop.objectReferences
                .Where(o => o != null && PrismProjectConfig.IsPrismSourceAssetPath(AssetDatabase.GetAssetPath(o)))
                .ToArray();

            if (mnAssets.Length == 0)
                return DragAndDropVisualMode.None;

            if (!perform)
                return DragAndDropVisualMode.Link;

            foreach (var mnAsset in mnAssets)
            {
                AddPrSMComponent(go, AssetDatabase.GetAssetPath(mnAsset));
            }

            return DragAndDropVisualMode.Link;
        }

        /// <summary>
        /// Add the generated MonoBehaviour component from a .prsm file to a GameObject.
        /// </summary>
        public static bool AddPrSMComponent(GameObject go, string mnAssetPath)
        {
            string className = Path.GetFileNameWithoutExtension(mnAssetPath);
            MonoScript script = FindGeneratedScript(className);

            if (script == null)
            {
                Debug.LogWarning($"[PrSM] Generated script not found for '{className}'. Build the project first (PrSM > Build Project).");
                return false;
            }

            Type scriptType = script.GetClass();
            if (scriptType == null)
            {
                Debug.LogWarning($"[PrSM] Type '{className}' not found. The generated C# may have compilation errors.");
                return false;
            }

            if (!typeof(MonoBehaviour).IsAssignableFrom(scriptType))
            {
                Debug.LogWarning($"[PrSM] '{className}' is not a MonoBehaviour and cannot be added as a component.");
                return false;
            }

            Undo.AddComponent(go, scriptType);
            Debug.Log($"[PrSM] Added component '{className}' to {go.name}");
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
        /// - .prsm file → open in VSCode
        /// - Generated .cs file → redirect to corresponding .prsm file
        /// - Inspector script field double-click → redirect to .prsm if exists
        /// </summary>
        [OnOpenAsset(0)] // Priority 0 = runs before default handler
        #pragma warning disable CS0618 // OnOpenAsset requires int instanceID
        public static bool OnOpenPrSMAsset(int instanceID, int line)
        {
            var obj = EditorUtility.InstanceIDToObject(instanceID);
        #pragma warning restore CS0618
            if (obj == null)
            {
                return PrismConsoleRemapOpener.TryOpenSelectedRemappedFrame(PrismProjectSettings.GetProjectRoot());
            }
            string path = AssetDatabase.GetAssetPath(obj);

            // Case 1: Direct source file double-click (also triggered when user clicks a remapped console log)
            if (PrismProjectConfig.IsPrismSourceAssetPath(path))
            {
                string projectRoot = PrismProjectSettings.GetProjectRoot();
                string fullPath = Path.Combine(projectRoot, path);
                int sourceLine = Math.Max(1, line);
                int sourceCol = 1;

                // Prefer the location cache populated by PrismRuntimeStackTraceRemapper when the
                // log was emitted — m_ActiveText reflection is unreliable at click time because
                // the console loses focus before OnOpenAsset fires.
                if (!PrismRuntimeStackTraceRemapper.TryConsumeCachedLocation(fullPath, out sourceLine, out sourceCol))
                {
                    // Fallback: try reading m_ActiveText (works for direct double-click with known line)
                    if (!PrismConsoleRemapOpener.TryGetSelectedLocationForAsset(projectRoot, fullPath, out sourceLine, out sourceCol))
                    {
                        sourceLine = Math.Max(1, line);
                        sourceCol = 1;
                    }
                }

                OpenInEditor(fullPath, sourceLine, sourceCol);
                return true;
            }

            // Case 2: Generated .cs file — redirect to .prsm source
            if (path.EndsWith(".cs"))
            {
                string projectRoot = PrismProjectSettings.GetProjectRoot();
                string outputDir = PrismProjectSettings.GetOutputDir();
                if (path.StartsWith(outputDir) || path.Contains("com.prsm.generated"))
                {
                    string fullGeneratedPath = Path.Combine(projectRoot, path);
                    if (PrismSourceMap.TryResolveSourceLocation(projectRoot, fullGeneratedPath, line, out string sourcePath, out int sourceLine, out int sourceCol))
                    {
                        OpenInEditor(sourcePath, sourceLine, sourceCol);
                        return true;
                    }

                    string className = Path.GetFileNameWithoutExtension(path);
                    string mnPath = FindPrSMSource(className);
                    if (mnPath != null)
                    {
                        OpenInEditor(Path.Combine(projectRoot, mnPath), line, 1);
                        return true;
                    }
                }
            }

            return false; // Let Unity handle it
        }

        /// <summary>
        /// Find the .prsm source file for a given class name.
        /// </summary>
        private static string FindPrSMSource(string className)
        {
            string[] guids = AssetDatabase.FindAssets(className + " t:TextAsset");
            foreach (string guid in guids)
            {
                string p = AssetDatabase.GUIDToAssetPath(guid);
                if (PrismProjectConfig.IsPrismSourceAssetPath(p) && Path.GetFileNameWithoutExtension(p) == className)
                    return p;
            }

            // Brute search in Assets
            string[] mnFiles = AssetDatabase.FindAssets("t:DefaultAsset");
            foreach (string guid in mnFiles)
            {
                string p = AssetDatabase.GUIDToAssetPath(guid);
                if (PrismProjectConfig.IsPrismSourceAssetPath(p) && Path.GetFileNameWithoutExtension(p) == className)
                    return p;
            }

            return null;
        }

        private static void OpenInEditor(string fullPath, int line, int col)
        {
            PrismEditorLauncher.OpenInEditor(fullPath, line, col);
        }
    }
}
