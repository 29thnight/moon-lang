using System.IO;
using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismStackTraceFormatterTests
    {
        [Test]
        public void TryRemapStackTraceLine_RemapsUnityAtFrameToPrSMSource()
        {
            string projectRoot = CreateProjectRoot();

            try
            {
                string line = "Player.Update() (at Packages/com.prsm.generated/Runtime/Player.cs:19)";

                bool remapped = PrismStackTraceFormatter.TryRemapStackTraceLine(projectRoot, line, out string remappedLine);

                Assert.IsTrue(remapped);
                Assert.AreEqual(
                    "Player.Update() (at Assets/Player.prsm:8) [PrSM col 10]",
                    remappedLine);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void TryRemapStackTraceLine_RemapsDotNetFrameToPrSMSource()
        {
            string projectRoot = CreateProjectRoot();

            try
            {
                string generatedFile = Path.Combine(projectRoot, "Packages", "com.prsm.generated", "Runtime", "Player.cs");
                string line = $"at Player.Update() in {generatedFile}:line 19";

                bool remapped = PrismStackTraceFormatter.TryRemapStackTraceLine(projectRoot, line, out string remappedLine);

                Assert.IsTrue(remapped);
                Assert.AreEqual(
                    "at Player.Update() in Assets/Player.prsm:line 8 [PrSM col 10]",
                    remappedLine);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void FormatRemappedRuntimeMessage_ReturnsNullWhenStackTraceHasNoPrSMFrames()
        {
            string message = PrismStackTraceFormatter.FormatRemappedRuntimeMessage(
                @"C:\PrismProject",
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
                string message = PrismStackTraceFormatter.FormatRemappedRuntimeMessage(
                    projectRoot,
                    "NullReferenceException: sample",
                    "Player.Update() (at Packages/com.prsm.generated/Runtime/Player.cs:19)");

                Assert.AreEqual(
                    "Assets/Player.prsm(8,10): error [PrSMRuntime] NullReferenceException: sample\n" +
                    "[PrSM] Remapped runtime stack trace from generated PrSM C#\n" +
                    "Player.Update() (at Assets/Player.prsm:8) [PrSM col 10]",
                    message);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        [Test]
        public void TryExtractFirstPrSMLocation_ParsesClickableDiagnosticHeader()
        {
            bool parsed = PrismStackTraceFormatter.TryExtractFirstPrSMLocation(
                "Assets/Player.prsm(8,10): error [PrSMRuntime] NullReferenceException: sample\n" +
                "[PrSM] Remapped runtime stack trace from generated PrSM C#",
                out string sourcePath,
                out int sourceLine,
                out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets/Player.prsm", sourcePath);
            Assert.AreEqual(8, sourceLine);
            Assert.AreEqual(10, sourceCol);
        }

        [Test]
        public void TryRemapStackTraceLine_PrefersNestedStatementSegmentSourceLocation()
        {
            string projectRoot = CreateProjectRoot(includeNestedSegment: true);

            try
            {
                string line = "Player.Update() (at Packages/com.prsm.generated/Runtime/Player.cs:19)";

                bool remapped = PrismStackTraceFormatter.TryRemapStackTraceLine(projectRoot, line, out string remappedLine);

                Assert.IsTrue(remapped);
                Assert.AreEqual(
                    "Player.Update() (at Assets/Player.prsm:9) [PrSM col 13]",
                    remappedLine);
            }
            finally
            {
                Directory.Delete(projectRoot, true);
            }
        }

        private static string CreateProjectRoot(bool includeNestedSegment = false)
        {
            string projectRoot = Path.Combine(Path.GetTempPath(), "PrismStackTraceFormatterTests", Path.GetRandomFileName());
            string sourceFile = Path.Combine(projectRoot, "Assets", "Player.prsm");
            string generatedFile = Path.Combine(projectRoot, "Packages", "com.prsm.generated", "Runtime", "Player.cs");
            string sourceMapFile = PrismSourceMap.GetSourceMapPath(generatedFile);

            Directory.CreateDirectory(Path.GetDirectoryName(sourceFile));
            Directory.CreateDirectory(Path.GetDirectoryName(generatedFile));
            File.WriteAllText(sourceFile, "component Player : MonoBehaviour {}\n");
            File.WriteAllText(generatedFile, "// generated\n");
                        File.WriteAllText(sourceMapFile, includeNestedSegment ? @"{
    ""version"": 1,
    ""source_file"": ""Assets/Player.prsm"",
    ""generated_file"": ""Packages/com.prsm.generated/Runtime/Player.cs"",
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
            ""generated_name_span"": { ""line"": 18, ""col"": 17, ""end_line"": 18, ""end_col"": 22 },
            ""segments"": [
                {
                    ""kind"": ""statement"",
                    ""name"": ""stmt1"",
                    ""qualified_name"": ""Player.Update#stmt1"",
                    ""source_span"": { ""line"": 9, ""col"": 13, ""end_line"": 9, ""end_col"": 24 },
                    ""generated_span"": { ""line"": 19, ""col"": 1, ""end_line"": 19, ""end_col"": 32 }
                }
            ]
        }
    ]
}" : @"{
  ""version"": 1,
  ""source_file"": ""Assets/Player.prsm"",
  ""generated_file"": ""Packages/com.prsm.generated/Runtime/Player.cs"",
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