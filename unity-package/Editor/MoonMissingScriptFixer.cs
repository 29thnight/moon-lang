using System;
using System.Linq;
using UnityEngine;
using UnityEditor;

namespace Moon.Editor
{
    /// <summary>
    /// Shows a cleanup button below the GameObject Inspector header
    /// when Missing Script components are detected.
    /// Button click removes all Missing Scripts.
    /// Auto-hides when no Missing Scripts remain.
    /// </summary>
    [InitializeOnLoad]
    public static class MoonMissingScriptFixer
    {
        static MoonMissingScriptFixer()
        {
            UnityEditor.Editor.finishedDefaultHeaderGUI += OnHeaderGUI;
        }

        private static void OnHeaderGUI(UnityEditor.Editor editor)
        {
            if (editor == null) return;
            if (editor.GetType().Name != "GameObjectInspector") return;

            var go = editor.target as GameObject;
            if (go == null) return;

            int missingCount = go.GetComponents<Component>().Count(c => c == null);
            if (missingCount == 0) return;

            EditorGUILayout.Space(2);

            Color prev = GUI.backgroundColor;
            GUI.backgroundColor = new Color(1f, 0.55f, 0.1f, 0.3f);

            var style = new GUIStyle(GUI.skin.button)
            {
                fontStyle = FontStyle.Bold,
                fontSize = 11,
                fixedHeight = 44,
                normal = { textColor = new Color(1f, 0.75f, 0.35f) },
                hover = { textColor = new Color(1f, 0.85f, 0.5f) },
                active = { textColor = Color.white }
            };

            string label = missingCount == 1
                ? "\u26a0  1 Missing Script — Clean Up"
                : $"\u26a0  {missingCount} Missing Scripts — Clean Up";

            if (GUILayout.Button(label, style))
            {
                Undo.RegisterCompleteObjectUndo(go, "Clean Up Missing Scripts");
                int removed = GameObjectUtility.RemoveMonoBehavioursWithMissingScript(go);
                Debug.Log($"[Moon] Removed {removed} missing script(s) from {go.name}");
            }

            GUI.backgroundColor = prev;
        }

        [MenuItem("GameObject/Moon/Clean Up Missing Scripts", false, 50)]
        public static void CleanUpMenu()
        {
            foreach (var go in Selection.gameObjects)
            {
                int removed = GameObjectUtility.RemoveMonoBehavioursWithMissingScript(go);
                if (removed > 0)
                    Debug.Log($"[Moon] Removed {removed} missing script(s) from {go.name}");
            }
        }

        [MenuItem("GameObject/Moon/Clean Up Missing Scripts", true)]
        public static bool CleanUpValidate()
        {
            return Selection.gameObjects.Any(go => go.GetComponents<Component>().Any(c => c == null));
        }
    }
}
