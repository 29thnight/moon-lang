using System.IO;
using UnityEngine;
using UnityEditor;

namespace Prism.Editor
{
    /// <summary>
    /// Custom editor for ALL MonoBehaviours.
    /// - Normal script: default Inspector
    /// - PrSM script: shows .prsm reference, click → ping/open .prsm
    /// - Missing script (null): hides default content, shows drop zone for .prsm or MonoScript
    /// </summary>
    [CustomEditor(typeof(MonoBehaviour), true)]
    [CanEditMultipleObjects]
    public class PrismComponentEditor : UnityEditor.Editor
    {
        private MonoScript _script;
        private string _mnPath;
        private bool _isPrSMGenerated;
        private UnityEngine.Object _mnAsset;
        private void OnEnable()
        {
            _isPrSMGenerated = false;

            if (target == null || !(target is MonoBehaviour mb))
                return;

            _script = MonoScript.FromMonoBehaviour(mb);
            if (_script == null)
                return;

            string csPath = AssetDatabase.GetAssetPath(_script);
            _isPrSMGenerated = csPath.Contains("com.prsm.generated");

            if (_isPrSMGenerated)
            {
                string className = _script.name;
                string[] guids = AssetDatabase.FindAssets(className);
                foreach (string guid in guids)
                {
                    string p = AssetDatabase.GUIDToAssetPath(guid);
                    if (PrismProjectConfig.IsPrismSourceAssetPath(p) && Path.GetFileNameWithoutExtension(p) == className)
                    {
                        _mnPath = p;
                        _mnAsset = AssetDatabase.LoadAssetAtPath<UnityEngine.Object>(p);
                        break;
                    }
                }
            }
        }

        public override void OnInspectorGUI()
        {
            // === PrSM Script → custom .prsm reference ===
            if (_isPrSMGenerated && _mnAsset != null)
            {
                DrawPrSMScriptInspector();
                return;
            }

            // === Normal Script → default ===
            DrawDefaultInspector();
        }

        private void DrawPrSMScriptInspector()
        {
            // Script field → shows .prsm
            EditorGUI.BeginDisabledGroup(true);
            EditorGUILayout.ObjectField("Script", _mnAsset, typeof(UnityEngine.Object), false);
            EditorGUI.EndDisabledGroup();

            // Click handling
            Rect lastRect = GUILayoutUtility.GetLastRect();
            Event evt = Event.current;
            if (evt.type == EventType.MouseDown && lastRect.Contains(evt.mousePosition))
            {
                if (evt.clickCount == 1)
                {
                    EditorGUIUtility.PingObject(_mnAsset);
                    evt.Use();
                }
                else if (evt.clickCount == 2)
                {
                    string fullPath = Path.Combine(PrismProjectSettings.GetProjectRoot(), _mnPath);
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
                    evt.Use();
                }
            }

            // Remaining properties
            serializedObject.Update();
            SerializedProperty prop = serializedObject.GetIterator();
            prop.NextVisible(true);
            while (prop.NextVisible(false))
            {
                if (prop.name == "m_Script") continue;
                EditorGUILayout.PropertyField(prop, true);
            }
            serializedObject.ApplyModifiedProperties();
        }
    }
}
