using System.IO;
using NUnit.Framework;

namespace Moon.Editor.Tests
{
    public class MoonStackTraceFormatterTests
    {
        [Test]
        public void TryRemapStackTraceLine_RemapsUnityAtFrameToMoonSource()
        {
            string projectRoot = CreateProjectRoot();

            try
            {
                string line = "Player.Update() (at Packages/com.moon.generated/Runtime/Player.cs:19)";

                bool remapped = MoonStackTraceFormatter.TryRemapStackTraceLine(projectRoot, line, out string remappedLine);

                Assert.IsTrue(remapped);
                Assert.AreEqual(
                    "Player.Update() (at Assets/Player.mn:8) [Moon col 10]",
                    remappedLine);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void TryRemapStackTraceLine_RemapsDotNetFrameToMoonSource()
        {
            string projectRoot = CreateProjectRoot();

            try
            {
                string generatedFile = Path.Combine(projectRoot, "Packages", "com.moon.generated", "Runtime", "Player.cs");
                string line = $"at Player.Update() in {generatedFile}:line 19";

                bool remapped = MoonStackTraceFormatter.TryRemapStackTraceLine(projectRoot, line, out string remappedLine);

                Assert.IsTrue(remapped);
                Assert.AreEqual(
                    "at Player.Update() in Assets/Player.mn:line 8 [Moon col 10]",
                    remappedLine);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void FormatRemappedRuntimeMessage_ReturnsNullWhenStackTraceHasNoMoonFrames()
        {
            string message = MoonStackTraceFormatter.FormatRemappedRuntimeMessage(
                @"C:\MoonProject",
                "NullReferenceException: sample",
                "Player.Update() (at Assets/Scripts/Player.cs:12)");

            Assert.IsNull(message);
        }

        [Test]
        public void FormatRemappedRuntimeMessage_IncludesClickableSummaryAndRemappedFrames()
        {
            string projectRoot = CreateProjectRoot();

            try
            {
                string message = MoonStackTraceFormatter.FormatRemappedRuntimeMessage(
                    projectRoot,
                    "NullReferenceException: sample",
                    "Player.Update() (at Packages/com.moon.generated/Runtime/Player.cs:19)");

                Assert.AreEqual(
                    "Assets/Player.mn(8,10): error [MoonRuntime] NullReferenceException: sample\n" +
                    "[Moon] Remapped runtime stack trace from generated Moon C#\n" +
                    "Player.Update() (at Assets/Player.mn:8) [Moon col 10]",
                    message);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        private static string CreateProjectRoot()
        {
            string projectRoot = Path.Combine(Path.GetTempPath(), "MoonStackTraceFormatterTests", Path.GetRandomFileName());
            string sourceFile = Path.Combine(projectRoot, "Assets", "Player.mn");
            string generatedFile = Path.Combine(projectRoot, "Packages", "com.moon.generated", "Runtime", "Player.cs");
            string sourceMapFile = MoonSourceMap.GetSourceMapPath(generatedFile);

            Directory.CreateDirectory(Path.GetDirectoryName(sourceFile));
            Directory.CreateDirectory(Path.GetDirectoryName(generatedFile));
            File.WriteAllText(sourceFile, "component Player : MonoBehaviour {}\n");
            File.WriteAllText(generatedFile, "// generated\n");
            File.WriteAllText(sourceMapFile, @"{
  ""version"": 1,
  ""source_file"": ""Assets/Player.mn"",
  ""generated_file"": ""Packages/com.moon.generated/Runtime/Player.cs"",
  ""declaration"": {
    ""kind"": ""type"",
    ""name"": ""Player"",
    ""qualified_name"": ""Player"",
    ""source_span"": { ""line"": 1, ""col"": 11, ""end_line"": 1, ""end_col"": 16 },
    ""generated_span"": { ""line"": 7, ""col"": 1, ""end_line"": 23, ""end_col"": 1 },
    ""generated_name_span"": { ""line"": 7, ""col"": 14, ""end_line"": 7, ""end_col"": 19 }
  },
  ""members"": [
    {
      ""kind"": ""function"",
      ""name"": ""Update"",
      ""qualified_name"": ""Player.Update"",
      ""source_span"": { ""line"": 8, ""col"": 10, ""end_line"": 8, ""end_col"": 15 },
      ""generated_span"": { ""line"": 18, ""col"": 1, ""end_line"": 22, ""end_col"": 5 },
      ""generated_name_span"": { ""line"": 18, ""col"": 17, ""end_line"": 18, ""end_col"": 22 }
    }
  ]
}");

            return projectRoot;
        }
    }
}