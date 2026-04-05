using UnityEngine;

namespace Prism
{
    /// <summary>
    /// Represents a PrSM (.prsm) script asset in Unity.
    /// This is the asset type shown in the Inspector when selecting a .prsm file.
    /// </summary>
    [Icon("Packages/com.prsm.editor/Editor/Icons/prsm-script-icon.png")]
    public class PrismScript : ScriptableObject
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
