using System;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEditor;
using UnityEditor.AssetImporters;

namespace Moon.Editor
{
    /// <summary>
    /// ScriptedImporter for .mn files.
    /// - Custom icon in Project window
    /// - Compiles to C# on import
    /// - Links to generated MonoScript for AddComponent + drag-and-drop
    /// </summary>
    [ScriptedImporter(3, "mn")]
    public class MoonImporter : ScriptedImporter
    {
        private static Texture2D _cachedIcon;

        public override void OnImportAsset(AssetImportContext ctx)
        {
            string moonPath = ctx.assetPath;
            string projectRoot = MoonProjectSettings.GetProjectRoot();

            // Ensure generated package exists
            MoonProjectSettings.EnsureGeneratedPackage();

            string outputDir = MoonProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(projectRoot, outputDir);

            if (!Directory.Exists(fullOutputDir))
                Directory.CreateDirectory(fullOutputDir);

            // Read source
            string fullMoonPath = Path.Combine(projectRoot, moonPath);
            string sourceText = File.ReadAllText(fullMoonPath);

            // Compile
            var result = MoonCompilerBridge.CompileFile(fullMoonPath, fullOutputDir);

            string className = Path.GetFileNameWithoutExtension(moonPath);

            if (result.Success)
            {
                string csRelPath = Path.Combine(outputDir, className + ".cs");
                Debug.Log($"[Moon] Compiled {moonPath} → {csRelPath}");

                var moonScript = ScriptableObject.CreateInstance<MoonScript>();
                moonScript.name = className;
                moonScript.SetData(className, sourceText, csRelPath);
                ctx.AddObjectToAsset("moon-script", moonScript, GetMoonIcon());
                ctx.SetMainObject(moonScript);

                string csPathCopy = csRelPath;
                EditorApplication.delayCall += () =>
                {
                    AssetDatabase.ImportAsset(csPathCopy, ImportAssetOptions.ForceUpdate);
                    AssetDatabase.Refresh();
                };
            }
            else
            {
                MoonCompilerBridge.LogDiagnostics(result, moonPath);

                var moonScript = ScriptableObject.CreateInstance<MoonScript>();
                moonScript.name = className;
                moonScript.SetData(className, sourceText, "");
                ctx.AddObjectToAsset("moon-script", moonScript, GetMoonIcon());
                ctx.SetMainObject(moonScript);
            }
        }

        /// <summary>
        /// Load the Moon script icon from the package.
        /// </summary>
        private static Texture2D GetMoonIcon()
        {
            if (_cachedIcon != null) return _cachedIcon;

            // Search in package
            string[] searchPaths = {
                "Packages/com.moon.editor/Editor/Icons/moon-script-icon.png",
                "Assets/Plugins/Moon/Editor/Icons/moon-script-icon.png",
            };

            foreach (string p in searchPaths)
            {
                _cachedIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(p);
                if (_cachedIcon != null) return _cachedIcon;
            }

            // Fallback: find by name
            string[] guids = AssetDatabase.FindAssets("moon-script-icon t:Texture2D");
            if (guids.Length > 0)
            {
                string path = AssetDatabase.GUIDToAssetPath(guids[0]);
                _cachedIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(path);
            }

            return _cachedIcon;
        }
    }
}
