using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Simple input dialog for naming scripts.
    /// </summary>
    public class EditorInputDialog : EditorWindow
    {
        private string _input = "";
        private string _label = "";
        private bool _confirmed;
        private bool _firstFrame = true;

        private static string _result;

        public static string Show(string title, string label, string defaultValue)
        {
            _result = null;

            var window = CreateInstance<EditorInputDialog>();
            window.titleContent = new GUIContent(title);
            window._input = defaultValue;
            window._label = label;
            window.minSize = new Vector2(320, 100);
            window.maxSize = new Vector2(320, 100);
            window.ShowModalUtility();

            return _result;
        }

        private void OnGUI()
        {
            EditorGUILayout.Space(8);
            EditorGUILayout.LabelField(_label);

            GUI.SetNextControlName("InputField");
            _input = EditorGUILayout.TextField(_input);

            if (_firstFrame)
            {
                EditorGUI.FocusTextInControl("InputField");
                _firstFrame = false;
            }

            // Enter key
            if (Event.current.type == EventType.KeyDown && Event.current.keyCode == KeyCode.Return)
            {
                _result = _input;
                Close();
                return;
            }

            // Escape key
            if (Event.current.type == EventType.KeyDown && Event.current.keyCode == KeyCode.Escape)
            {
                Close();
                return;
            }

            EditorGUILayout.Space(4);
            EditorGUILayout.BeginHorizontal();
            GUILayout.FlexibleSpace();
            if (GUILayout.Button("Create", GUILayout.Width(80)))
            {
                _result = _input;
                Close();
            }
            if (GUILayout.Button("Cancel", GUILayout.Width(80)))
            {
                Close();
            }
            EditorGUILayout.EndHorizontal();
        }
    }
}
