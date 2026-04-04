using System.IO;
using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Adds Moon script creation items to Assets > Create > Moon menu.
    /// </summary>
    public static class MoonMenuItems
    {
        private const int MenuPriority = 80;

        [MenuItem("Assets/Create/Moon/Component", priority = MenuPriority)]
        public static void CreateComponent()
        {
            CreateMoonFile("NewComponent", "Component");
        }

        [MenuItem("Assets/Create/Moon/MonoBehaviour", priority = MenuPriority + 1)]
        public static void CreateMonoBehaviour()
        {
            CreateMoonFile("NewMonoBehaviour", "MonoBehaviour");
        }

        [MenuItem("Assets/Create/Moon/ScriptableObject", priority = MenuPriority + 2)]
        public static void CreateScriptableObject()
        {
            CreateMoonFile("NewAsset", "ScriptableObject");
        }

        [MenuItem("Assets/Create/Moon/PlayableAsset", priority = MenuPriority + 3)]
        public static void CreatePlayableAsset()
        {
            CreateMoonFile("NewPlayableAsset", "PlayableAsset");
        }

        [MenuItem("Assets/Create/Moon/PlayableBehaviour", priority = MenuPriority + 4)]
        public static void CreatePlayableBehaviour()
        {
            CreateMoonFile("NewPlayableBehaviour", "PlayableBehaviour");
        }

        [MenuItem("Assets/Create/Moon/C# Class", priority = MenuPriority + 5)]
        public static void CreateClass()
        {
            CreateMoonFile("NewClass", "CSharpClass");
        }

        [MenuItem("Moon/Build Project", priority = 100)]
        public static void BuildProject()
        {
            MoonProjectSettings.EnsureProjectFile();

            var result = MoonCompilerBridge.BuildProject();
            if (result.Success)
            {
                Debug.Log("[Moon] Build succeeded.");
                AssetDatabase.Refresh();
                MoonIconAssigner.AssignIconsToGeneratedScripts();
            }
            else
            {
                MoonCompilerBridge.LogDiagnostics(result);
            }
        }

        [MenuItem("Moon/Refresh .mnproject Cache", priority = 101)]
        public static void RefreshCache()
        {
            MoonProjectSettings.ClearCache();
            Debug.Log("[Moon] Settings cache cleared.");
        }

        private static void CreateMoonFile(string defaultName, string templateName)
        {
            string scriptName = EditorInputDialog.Show("New Moon Script", "Script name:", defaultName);
            if (string.IsNullOrEmpty(scriptName))
            {
                return;
            }

            scriptName = scriptName.Replace(" ", "").Replace("-", "_");
            MoonProjectSettings.EnsureProjectFile();

            string outputDir = MoonProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(MoonProjectSettings.GetProjectRoot(), outputDir);
            if (!Directory.Exists(fullOutputDir))
            {
                Directory.CreateDirectory(fullOutputDir);
            }

            string folderPath = GetSelectedFolder();
            string template = MoonTemplates.GetTemplate(templateName);
            string filePath = AssetDatabase.GenerateUniqueAssetPath(Path.Combine(folderPath, scriptName + ".mn"));

            scriptName = Path.GetFileNameWithoutExtension(filePath);
            string content = template.Replace("#SCRIPTNAME#", scriptName);
            string fullPath = Path.Combine(MoonProjectSettings.GetProjectRoot(), filePath);
            File.WriteAllText(fullPath, content);

            Debug.Log($"[Moon] Created {filePath}");

            var result = MoonCompilerBridge.CompileFile(fullPath, fullOutputDir);
            if (result.Success)
            {
                Debug.Log($"[Moon] Compiled {filePath} -> {Path.Combine(outputDir, scriptName + ".cs")}");
            }
            else
            {
                MoonCompilerBridge.LogDiagnostics(result, filePath);
            }

            AssetDatabase.Refresh();
            EditorApplication.delayCall += () => MoonIconAssigner.AssignIconToScript(scriptName);

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
