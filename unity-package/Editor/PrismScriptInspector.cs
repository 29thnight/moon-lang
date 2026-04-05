using UnityEngine;
using UnityEditor;

namespace Prism.Editor
{
    /// <summary>
    /// Custom Inspector for PrismScript assets.
    /// Shows script info and source preview.
    /// </summary>
    [CustomEditor(typeof(PrismScript))]
    public class PrismScriptInspector : UnityEditor.Editor
    {
        private bool _showSource = false;
        private Vector2 _scrollPos;

        public override void OnInspectorGUI()
        {
            var prsmScript = (PrismScript)target;

            // Header
            EditorGUILayout.LabelField("PrSM Script", EditorStyles.boldLabel);
            EditorGUILayout.Space(4);

            // Script name
            EditorGUILayout.LabelField("Script Name", prsmScript.ScriptName);

            // Generated C# path
            if (!string.IsNullOrEmpty(prsmScript.GeneratedCsPath))
            {
                EditorGUILayout.LabelField("Generated C#", prsmScript.GeneratedCsPath);
            }

            EditorGUILayout.Space(8);

            // Open in Editor button
            EditorGUILayout.BeginHorizontal();
            if (GUILayout.Button("Open in VSCode", GUILayout.Height(28)))
            {
                string assetPath = AssetDatabase.GetAssetPath(target);
                string fullPath = System.IO.Path.Combine(
                    PrismProjectSettings.GetProjectRoot(), assetPath);

                try
                {
                    System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
                    {
                        FileName = "code",
                        Arguments = $"--goto \"{fullPath}\"",
                        UseShellExecute = true,
                        CreateNoWindow = true
                    });
                }
                catch
                {
                    EditorUtility.OpenWithDefaultApp(fullPath);
                }
            }

            if (GUILayout.Button("Recompile", GUILayout.Height(28)))
            {
                string assetPath = AssetDatabase.GetAssetPath(target);
                AssetDatabase.ImportAsset(assetPath, ImportAssetOptions.ForceUpdate);
            }
            EditorGUILayout.EndHorizontal();

            EditorGUILayout.Space(8);

            // Source code preview
            _showSource = EditorGUILayout.Foldout(_showSource, "Source Code Preview");
            if (_showSource && !string.IsNullOrEmpty(prsmScript.SourceCode))
            {
                EditorGUILayout.Space(4);
                _scrollPos = EditorGUILayout.BeginScrollView(_scrollPos, GUILayout.MaxHeight(400));
                EditorGUI.BeginDisabledGroup(true);
                EditorGUILayout.TextArea(prsmScript.SourceCode, GUILayout.ExpandHeight(true));
                EditorGUI.EndDisabledGroup();
                EditorGUILayout.EndScrollView();
            }
        }
    }
}
