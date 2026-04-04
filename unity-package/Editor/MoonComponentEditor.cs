using System.IO;
using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Custom editor for ALL MonoBehaviours.
    /// - Normal script: default Inspector
    /// - Moon script: shows .mn reference, click → ping/open .mn
    /// - Missing script (null): hides default content, shows drop zone for .mn or MonoScript
    /// </summary>
    [CustomEditor(typeof(MonoBehaviour), true)]
    [CanEditMultipleObjects]
    public class MoonComponentEditor : UnityEditor.Editor
    {
        private MonoScript _script;
        private string _mnPath;
        private bool _isMoonGenerated;
        private UnityEngine.Object _mnAsset;
        private void OnEnable()
        {
            _isMoonGenerated = false;

            if (target == null || !(target is MonoBehaviour mb))
                return;

            _script = MonoScript.FromMonoBehaviour(mb);
            if (_script == null)
                return;

            string csPath = AssetDatabase.GetAssetPath(_script);
            _isMoonGenerated = csPath.Contains("com.moon.generated");

            if (_isMoonGenerated)
            {
                string className = _script.name;
                string[] guids = AssetDatabase.FindAssets(className);
                foreach (string guid in guids)
                {
                    string p = AssetDatabase.GUIDToAssetPath(guid);
                    if (p.EndsWith(".mn") && Path.GetFileNameWithoutExtension(p) == className)
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
            // === Moon Script → custom .mn reference ===
            if (_isMoonGenerated && _mnAsset != null)
            {
                DrawMoonScriptInspector();
                return;
            }

            // === Normal Script → default ===
            DrawDefaultInspector();
        }

        private void DrawMoonScriptInspector()
        {
            // Script field → shows .mn
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
                    string fullPath = Path.Combine(MoonProjectSettings.GetProjectRoot(), _mnPath);
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
