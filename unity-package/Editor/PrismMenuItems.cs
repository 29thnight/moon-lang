using System.IO;
using UnityEngine;
using UnityEditor;

namespace Prism.Editor
{
    /// <summary>
    /// Adds PrSM script creation items to Assets > Create > PrSM menu.
    /// </summary>
    public static class PrismMenuItems
    {
        private const int MenuPriority = 80;

        [MenuItem("Assets/Create/PrSM/Component", priority = MenuPriority)]
        public static void CreateComponent()
        {
            CreatePrSMFile("NewComponent", "Component");
        }

        [MenuItem("Assets/Create/PrSM/MonoBehaviour", priority = MenuPriority + 1)]
        public static void CreateMonoBehaviour()
        {
            CreatePrSMFile("NewMonoBehaviour", "MonoBehaviour");
        }

        [MenuItem("Assets/Create/PrSM/ScriptableObject", priority = MenuPriority + 2)]
        public static void CreateScriptableObject()
        {
            CreatePrSMFile("NewAsset", "ScriptableObject");
        }

        [MenuItem("Assets/Create/PrSM/PlayableAsset", priority = MenuPriority + 3)]
        public static void CreatePlayableAsset()
        {
            CreatePrSMFile("NewPlayableAsset", "PlayableAsset");
        }

        [MenuItem("Assets/Create/PrSM/PlayableBehaviour", priority = MenuPriority + 4)]
        public static void CreatePlayableBehaviour()
        {
            CreatePrSMFile("NewPlayableBehaviour", "PlayableBehaviour");
        }

        [MenuItem("Assets/Create/PrSM/C# Class", priority = MenuPriority + 5)]
        public static void CreateClass()
        {
            CreatePrSMFile("NewClass", "CSharpClass");
        }

        [MenuItem("PrSM/Build Project", priority = 100)]
        public static void BuildProject()
        {
            PrismProjectSettings.EnsureProjectFile();

            var result = PrismCompilerBridge.BuildProject();
            if (result.Success)
            {
                Debug.Log("[PrSM] Build succeeded.");
                AssetDatabase.Refresh();
                PrismIconAssigner.AssignIconsToGeneratedScripts();
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(result);
            }
        }

        [MenuItem("PrSM/Rebuild Project", priority = 101)]
        public static void RebuildProject()
        {
            PrismProjectSettings.EnsureProjectFile();

            var result = PrismCompilerBridge.RebuildProject();
            if (result.Success)
            {
                Debug.Log($"[PrSM] Rebuild succeeded — {result.Report.compiled} file(s) recompiled.");
                AssetDatabase.Refresh();
                PrismIconAssigner.AssignIconsToGeneratedScripts();
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(result);
            }
        }

        [MenuItem("PrSM/Refresh .prsmproject Cache", priority = 200)]
        public static void RefreshCache()
        {
            PrismProjectSettings.ClearCache();
            Debug.Log("[PrSM] Settings cache cleared.");
        }

        private static void CreatePrSMFile(string defaultName, string templateName)
        {
            string scriptName = EditorInputDialog.Show("New PrSM Script", "Script name:", defaultName);
            if (string.IsNullOrEmpty(scriptName))
            {
                return;
            }

            scriptName = scriptName.Replace(" ", "").Replace("-", "_");
            PrismProjectSettings.EnsureProjectFile();

            string outputDir = PrismProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(PrismProjectSettings.GetProjectRoot(), outputDir);
            if (!Directory.Exists(fullOutputDir))
            {
                Directory.CreateDirectory(fullOutputDir);
            }

            string folderPath = GetSelectedFolder();
            string template = PrismTemplates.GetTemplate(templateName);
            string filePath = AssetDatabase.GenerateUniqueAssetPath(Path.Combine(folderPath, scriptName + ".prsm"));

            scriptName = Path.GetFileNameWithoutExtension(filePath);
            string content = template.Replace("#SCRIPTNAME#", scriptName);
            string fullPath = Path.Combine(PrismProjectSettings.GetProjectRoot(), filePath);
            File.WriteAllText(fullPath, content);

            Debug.Log($"[PrSM] Created {filePath}");

            var result = PrismCompilerBridge.CompileFile(fullPath, fullOutputDir);
            if (result.Success)
            {
                Debug.Log($"[PrSM] Compiled {filePath} -> {Path.Combine(outputDir, scriptName + ".cs")}");
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(result, filePath);
            }

            AssetDatabase.Refresh();
            EditorApplication.delayCall += () => PrismIconAssigner.AssignIconToScript(scriptName);

            var asset = AssetDatabase.LoadAssetAtPath<Object>(filePath);
            if (asset != null)
            {
                Selection.activeObject = asset;
                EditorGUIUtility.PingObject(asset);
            }
        }

        private static string GetSelectedFolder()
        {
            string path = "Assets";

            foreach (Object obj in Selection.GetFiltered(typeof(Object), SelectionMode.Assets))
            {
                path = AssetDatabase.GetAssetPath(obj);
                if (!string.IsNullOrEmpty(path) && File.Exists(path))
                {
                    path = Path.GetDirectoryName(path);
                }
                break;
            }

            return path;
        }
    }
}
