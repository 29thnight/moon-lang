using System.IO;
using System.Text;
using NUnit.Framework;

namespace Prism.Editor.Tests
{
    public class PrismSourceMapTests
    {
        [Test]
        public void GetSourceMapPath_UsesMnmapSidecarExtension()
        {
            Assert.AreEqual(
                Path.Combine("Generated", "PrSM", "Player.prsmmap.json"),
                PrismSourceMap.GetSourceMapPath(Path.Combine("Generated", "PrSM", "Player.cs")));
        }

        [Test]
        public void FindAnchorForGeneratedPosition_PrefersMostSpecificMemberSpan()
        {
            var sourceMap = new PrSMGeneratedSourceMapFile
            {
                source_file = "Assets/Player.prsm",
                generated_file = "Generated/PrSM/Player.cs",
                declaration = new PrSMGeneratedSourceMapAnchor
                {
                    qualified_name = "Player",
                    source_span = new PrismGeneratedSourceMapSpan { line = 1, col = 11, end_line = 1, end_col = 16 },
                    generated_span = new PrismGeneratedSourceMapSpan { line = 7, col = 1, end_line = 23, end_col = 1 },
                    generated_name_span = new PrismGeneratedSourceMapSpan { line = 7, col = 14, end_line = 7, end_col = 19 },
                },
                members = new[]
                {
                    new PrSMGeneratedSourceMapAnchor
                    {
                        qualified_name = "Player.speed",
                        source_span = new PrismGeneratedSourceMapSpan { line = 2, col = 15, end_line = 2, end_col = 19 },
                        generated_span = new PrismGeneratedSourceMapSpan { line = 9, col = 1, end_line = 14, end_col = 5 },
                        generated_name_span = new PrismGeneratedSourceMapSpan { line = 10, col = 18, end_line = 10, end_col = 22 },
                    },
                    new PrSMGeneratedSourceMapAnchor
                    {
                        qualified_name = "Player.jump",
                        source_span = new PrismGeneratedSourceMapSpan { line = 8, col = 10, end_line = 8, end_col = 13 },
                        generated_span = new PrismGeneratedSourceMapSpan { line = 18, col = 1, end_line = 22, end_col = 5 },
                        generated_name_span = new PrismGeneratedSourceMapSpan { line = 18, col = 17, end_line = 18, end_col = 20 },
                        segments = new[]
                        {
                            new PrSMGeneratedSourceMapAnchor
                            {
                                qualified_name = "Player.jump#stmt1",
                                source_span = new PrismGeneratedSourceMapSpan { line = 9, col = 13, end_line = 9, end_col = 24 },
                                generated_span = new PrismGeneratedSourceMapSpan { line = 19, col = 1, end_line = 19, end_col = 32 },
                            },
                        },
                    },
                },
            };

            PrSMGeneratedSourceMapAnchor anchor = PrismSourceMap.FindAnchorForGeneratedPosition(sourceMap, 19, 1);

            Assert.NotNull(anchor);
            Assert.AreEqual("Player.jump#stmt1", anchor.qualified_name);
        }

        [Test]
        public void TryResolveSourceLocation_UsesSidecarAndProjectRelativeSourcePath()
        {
            string projectRoot = Path.Combine(Path.GetTempPath(), "PrismSourceMapTests", Path.GetRandomFileName());
            string sourceFile = Path.Combine(projectRoot, "Assets", "Player.prsm");
            string generatedFile = Path.Combine(projectRoot, "Generated", "PrSM", "Player.cs");
            string sourceMapFile = PrismSourceMap.GetSourceMapPath(generatedFile);

            Directory.CreateDirectory(Path.GetDirectoryName(sourceFile));
            Directory.CreateDirectory(Path.GetDirectoryName(generatedFile));
            File.WriteAllText(sourceFile, "component Player : MonoBehaviour {}\n");
            File.WriteAllText(generatedFile, "// generated\n");
            File.WriteAllText(sourceMapFile, @"{
  ""version"": 1,
  ""source_file"": ""Assets/Player.prsm"",
  ""generated_file"": ""Generated/PrSM/Player.cs"",
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
      ""name"": ""jump"",
      ""qualified_name"": ""Player.jump"",
      ""source_span"": { ""line"": 8, ""col"": 10, ""end_line"": 8, ""end_col"": 13 },
            ""generated_span"": { ""line"": 18, ""col"": 1, ""end_line"": 22, ""end_col"": 5 },
            ""generated_name_span"": { ""line"": 18, ""col"": 17, ""end_line"": 18, ""end_col"": 20 },
            ""segments"": [
                {
                    ""kind"": ""statement"",
                    ""name"": ""stmt1"",
                    ""qualified_name"": ""Player.jump#stmt1"",
                    ""source_span"": { ""line"": 9, ""col"": 13, ""end_line"": 9, ""end_col"": 24 },
                    ""generated_span"": { ""line"": 19, ""col"": 1, ""end_line"": 19, ""end_col"": 32 }
                }
            ]
    }
  ]
}");

            bool resolved = PrismSourceMap.TryResolveSourceLocation(projectRoot, generatedFile, 19, out string resolvedPath, out int resolvedLine, out int resolvedCol);

            Assert.IsTrue(resolved);
            Assert.AreEqual(sourceFile, resolvedPath);
            Assert.AreEqual(9, resolvedLine);
            Assert.AreEqual(13, resolvedCol);

            Directory.Delete(projectRoot, true);
        }

        [Test]
        public void LoadSourceMap_ParsesDeeplyNestedStatementSegments()
        {
            const int nestingDepth = 12;

            string projectRoot = Path.Combine(Path.GetTempPath(), "PrismSourceMapTests", Path.GetRandomFileName());
            string sourceFile = Path.Combine(projectRoot, "Assets", "Player.prsm");
            string generatedFile = Path.Combine(projectRoot, "Generated", "PrSM", "Player.cs");
            string sourceMapFile = PrismSourceMap.GetSourceMapPath(generatedFile);

            Directory.CreateDirectory(Path.GetDirectoryName(sourceFile));
            Directory.CreateDirectory(Path.GetDirectoryName(generatedFile));
            File.WriteAllText(sourceFile, "component Player : MonoBehaviour {}\n");
            File.WriteAllText(generatedFile, "// generated\n");
            File.WriteAllText(sourceMapFile, CreateNestedSourceMapJson(nestingDepth));

            PrSMGeneratedSourceMapFile sourceMap = PrismSourceMap.LoadSourceMap(generatedFile);
            PrSMGeneratedSourceMapAnchor anchor = PrismSourceMap.FindAnchorForGeneratedPosition(sourceMap, 18 + nestingDepth, 1);

            Assert.NotNull(sourceMap);
            Assert.NotNull(anchor);
            Assert.AreEqual($"Player.jump#stmt{nestingDepth}", anchor.qualified_name);
            Assert.NotNull(anchor.source_span);
            Assert.AreEqual(8 + nestingDepth, anchor.source_span.line);

            Directory.Delete(projectRoot, true);
        }

        private static string CreateNestedSourceMapJson(int depth)
        {
            var builder = new StringBuilder();
            builder.AppendLine("{");
            builder.AppendLine("  \"version\": 1,");
            builder.AppendLine("  \"source_file\": \"Assets/Player.prsm\",");
            builder.AppendLine("  \"generated_file\": \"Generated/PrSM/Player.cs\",");
            builder.AppendLine("  \"declaration\": {");
            builder.AppendLine("    \"kind\": \"type\",");
            builder.AppendLine("    \"name\": \"Player\",");
            builder.AppendLine("    \"qualified_name\": \"Player\",");
            builder.AppendLine("    \"source_span\": { \"line\": 1, \"col\": 11, \"end_line\": 1, \"end_col\": 16 },");
            builder.AppendLine("    \"generated_span\": { \"line\": 7, \"col\": 1, \"end_line\": 40, \"end_col\": 1 },");
            builder.AppendLine("    \"generated_name_span\": { \"line\": 7, \"col\": 14, \"end_line\": 7, \"end_col\": 19 }");
            builder.AppendLine("  },");
            builder.AppendLine("  \"members\": [");
            builder.AppendLine("    {");
            builder.AppendLine("      \"kind\": \"function\",");
            builder.AppendLine("      \"name\": \"jump\",");
            builder.AppendLine("      \"qualified_name\": \"Player.jump\",");
            builder.AppendLine("      \"source_span\": { \"line\": 8, \"col\": 10, \"end_line\": 8, \"end_col\": 13 },");
            builder.AppendLine($"      \"generated_span\": {{ \"line\": 18, \"col\": 1, \"end_line\": {18 + depth}, \"end_col\": 32 }},");
            builder.AppendLine("      \"generated_name_span\": { \"line\": 18, \"col\": 17, \"end_line\": 18, \"end_col\": 20 },");
            builder.AppendLine("      \"segments\": [");
            AppendNestedSegment(builder, 1, depth, 8, "Player.jump");
            builder.AppendLine("      ]");
            builder.AppendLine("    }");
            builder.AppendLine("  ]");
            builder.Append('}');
            return builder.ToString();
        }

        private static void AppendNestedSegment(StringBuilder builder, int level, int maxDepth, int baseSourceLine, string parentQualifiedName)
        {
            string indent = new string(' ', 6 + (level * 2));
            int sourceLine = baseSourceLine + level;
            int generatedLine = 18 + level;
            string qualifiedName = $"{parentQualifiedName}#stmt{level}";

            builder.AppendLine($"{indent}{{");
            builder.AppendLine($"{indent}  \"kind\": \"statement\",");
            builder.AppendLine($"{indent}  \"name\": \"stmt{level}\",");
            builder.AppendLine($"{indent}  \"qualified_name\": \"{qualifiedName}\",");
            builder.AppendLine($"{indent}  \"source_span\": {{ \"line\": {sourceLine}, \"col\": 13, \"end_line\": {sourceLine}, \"end_col\": 24 }},");
            builder.AppendLine($"{indent}  \"generated_span\": {{ \"line\": {generatedLine}, \"col\": 1, \"end_line\": {generatedLine}, \"end_col\": 32 }}{(level < maxDepth ? "," : string.Empty)}");

            if (level < maxDepth)
            {
                builder.AppendLine($"{indent}  \"segments\": [");
                AppendNestedSegment(builder, level + 1, maxDepth, baseSourceLine, qualifiedName);
                builder.AppendLine($"{indent}  ]");
            }

            builder.Append($"{indent}}}");
            if (level < maxDepth)
            {
                builder.AppendLine();
            }
        }
    }
}