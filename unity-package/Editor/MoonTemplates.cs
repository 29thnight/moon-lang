using System.IO;
using UnityEngine;

namespace Moon.Editor
{
    /// <summary>
    /// Provides Moon script templates.
    /// Reads from Templates/ folder in the package, falls back to hardcoded defaults.
    /// </summary>
    public static class MoonTemplates
    {
        public static string GetTemplate(string templateName)
        {
            // Try to load from package Templates/ folder
            string packagePath = GetPackagePath();
            if (!string.IsNullOrEmpty(packagePath))
            {
                string templateFile = Path.Combine(packagePath, "Templates", templateName + ".mn.txt");
                if (File.Exists(templateFile))
                {
                    return File.ReadAllText(templateFile);
                }
            }

            // Fallback to hardcoded templates
            return GetDefaultTemplate(templateName);
        }

        private static string GetPackagePath()
        {
            // UPM package path
            string path = "Packages/com.moon.editor";
            if (Directory.Exists(Path.Combine(Application.dataPath, "..", path)))
                return Path.Combine(Application.dataPath, "..", path);

            // Local development path
            string[] searchPaths = {
                Path.Combine(Application.dataPath, "..", "unity-package"),
                Path.Combine(Application.dataPath, "Plugins", "Moon"),
            };

            foreach (var p in searchPaths)
            {
                if (Directory.Exists(p)) return p;
            }

            return null;
        }

        private static string GetDefaultTemplate(string name)
        {
            switch (name)
            {
                case "Component":
                    return @"using UnityEngine

component #SCRIPTNAME# : MonoBehaviour {
    awake {
    }

    update {
    }
}
";
                case "MonoBehaviour":
                    return @"using UnityEngine

component #SCRIPTNAME# : MonoBehaviour {
    awake {
    }

    update {
    }
}
";
                case "ScriptableObject":
                    return @"using UnityEngine

asset #SCRIPTNAME# : ScriptableObject {
}
";
                case "PlayableAsset":
                    return @"using UnityEngine
using UnityEngine.Playables

component #SCRIPTNAME# : PlayableAsset {
    func createPlayable(graph: PlayableGraph, owner: GameObject): Playable {
        intrinsic {
            return ScriptPlayable<#SCRIPTNAME#Behaviour>.Create(graph);
        }
    }
}
";
                case "PlayableBehaviour":
                    return @"using UnityEngine.Playables

class #SCRIPTNAME# : PlayableBehaviour {
    func processFrame(playable: Playable, info: FrameData) {
    }
}
";
                case "CSharpClass":
                    return @"class #SCRIPTNAME# {
}
";
                default:
                    return $"// Moon script: #SCRIPTNAME#\n";
            }
        }
    }
}
