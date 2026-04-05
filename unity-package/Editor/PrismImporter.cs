using System;
using System.IO;
using System.Linq;
using UnityEngine;
using UnityEditor;
using UnityEditor.AssetImporters;

namespace Prism.Editor
{
    /// <summary>
    /// ScriptedImporter for PrSM source files.
    /// - Custom icon in Project window
    /// - Compiles to C# on import
    /// - Links to generated MonoScript for AddComponent + drag-and-drop
    /// </summary>
    [ScriptedImporter(3, new[] { "prsm", "mn" })]
    public class PrismImporter : ScriptedImporter
    {
        private static Texture2D _cachedIcon;

        public override void OnImportAsset(AssetImportContext ctx)
        {
            string prsmPath = ctx.assetPath;
            string projectRoot = PrismProjectSettings.GetProjectRoot();

            // Ensure generated package exists
            PrismProjectSettings.EnsureGeneratedPackage();

            string outputDir = PrismProjectSettings.GetOutputDir();
            string fullOutputDir = Path.Combine(projectRoot, outputDir);

            if (!Directory.Exists(fullOutputDir))
                Directory.CreateDirectory(fullOutputDir);

            // Read source
            string fullPrSMPath = Path.Combine(projectRoot, prsmPath);
            string sourceText = File.ReadAllText(fullPrSMPath);

            // Compile
            var result = PrismCompilerBridge.CompileFile(fullPrSMPath, fullOutputDir);

            string className = Path.GetFileNameWithoutExtension(prsmPath);

            if (result.Success)
            {
                string csRelPath = Path.Combine(outputDir, className + ".cs");
                Debug.Log($"[PrSM] Compiled {prsmPath} → {csRelPath}");

                var prsmScript = ScriptableObject.CreateInstance<PrismScript>();
                prsmScript.name = className;
                prsmScript.SetData(className, sourceText, csRelPath);
                ctx.AddObjectToAsset("prsm-script", prsmScript, GetPrSMIcon());
                ctx.SetMainObject(prsmScript);

                string csPathCopy = csRelPath;
                EditorApplication.delayCall += () =>
                {
                    AssetDatabase.ImportAsset(csPathCopy, ImportAssetOptions.ForceUpdate);
                    AssetDatabase.Refresh();
                };
            }
            else
            {
                PrismCompilerBridge.LogDiagnostics(result, prsmPath);

                var prsmScript = ScriptableObject.CreateInstance<PrismScript>();
                prsmScript.name = className;
                prsmScript.SetData(className, sourceText, "");
                ctx.AddObjectToAsset("prsm-script", prsmScript, GetPrSMIcon());
                ctx.SetMainObject(prsmScript);
            }
        }

        /// <summary>
        /// Load the PrSM script icon from the package.
        /// </summary>
        private static Texture2D GetPrSMIcon()
        {
            if (_cachedIcon != null) return _cachedIcon;

            // Search in package
            string[] searchPaths = {
                "Packages/com.prsm.editor/Editor/Icons/prsm-script-icon.png",
                "Assets/Plugins/PrSM/Editor/Icons/prsm-script-icon.png",
            };

            foreach (string p in searchPaths)
            {
                _cachedIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(p);
                if (_cachedIcon != null) return _cachedIcon;
            }

            // Fallback: find by name
            string[] guids = AssetDatabase.FindAssets("prsm-script-icon t:Texture2D");
            if (guids.Length > 0)
            {
                string path = AssetDatabase.GUIDToAssetPath(guids[0]);
                _cachedIcon = AssetDatabase.LoadAssetAtPath<Texture2D>(path);
            }

            return _cachedIcon;
        }
    }
}
