using UnityEngine;

namespace Moon
{
    /// <summary>
    /// Represents a Moon (.mn) script asset in Unity.
    /// This is the asset type shown in the Inspector when selecting a .mn file.
    /// </summary>
    [Icon("Packages/com.moon.editor/Editor/Icons/moon-script-icon.png")]
    public class MoonScript : ScriptableObject
    {
        [TextArea(10, 50)]
        [SerializeField] private string sourceCode;

        [SerializeField] private string scriptName;
        [SerializeField] private string generatedCsPath;

        public string SourceCode => sourceCode;
        public string ScriptName => scriptName;
        public string GeneratedCsPath => generatedCsPath;

        public void SetData(string name, string source, string csPath)
        {
            scriptName = name;
            sourceCode = source;
            generatedCsPath = csPath;
        }
    }
}
