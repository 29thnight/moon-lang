using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;

namespace Moon.Editor
{
    [Serializable]
    internal class MoonGeneratedSourceMapFile
    {
        public int version;
        public string source_file = string.Empty;
        public string generated_file = string.Empty;
        public MoonGeneratedSourceMapAnchor declaration;
        public MoonGeneratedSourceMapAnchor[] members = Array.Empty<MoonGeneratedSourceMapAnchor>();
    }

    [Serializable]
    internal class MoonGeneratedSourceMapAnchor
    {
        public string kind = string.Empty;
        public string name = string.Empty;
        public string qualified_name = string.Empty;
        public MoonGeneratedSourceMapSpan source_span;
        public MoonGeneratedSourceMapSpan generated_span;
        public MoonGeneratedSourceMapSpan generated_name_span;
        public MoonGeneratedSourceMapAnchor[] segments = Array.Empty<MoonGeneratedSourceMapAnchor>();
    }

    [Serializable]
    internal class MoonGeneratedSourceMapSpan
    {
        public int line;
        public int col;
        public int end_line;
        public int end_col;
    }

    internal static class MoonSourceMap
    {
        internal static string GetSourceMapPath(string generatedFilePath)
        {
            string directory = Path.GetDirectoryName(generatedFilePath) ?? string.Empty;
            string stem = Path.GetFileNameWithoutExtension(generatedFilePath);
            return Path.Combine(directory, stem + ".mnmap.json");
        }

        internal static MoonGeneratedSourceMapFile LoadSourceMap(string generatedFilePath)
        {
            string sourceMapPath = GetSourceMapPath(generatedFilePath);
            if (!File.Exists(sourceMapPath))
            {
                return null;
            }

            try
            {
                return MoonSourceMapJsonParser.Parse(File.ReadAllText(sourceMapPath));
            }
            catch
            {
                return null;
            }
        }

        internal static bool TryResolveSourceLocation(
            string projectRoot,
            string generatedFilePath,
            int generatedLine,
            out string sourcePath,
            out int sourceLine,
            out int sourceCol)
        {
            return TryResolveSourceLocation(
                projectRoot,
                generatedFilePath,
                generatedLine,
                1,
                out sourcePath,
                out sourceLine,
                out sourceCol);
        }

        internal static bool TryResolveSourceLocation(
            string projectRoot,
            string generatedFilePath,
            int generatedLine,
            int generatedCol,
            out string sourcePath,
            out int sourceLine,
            out int sourceCol)
        {
            generatedLine = Math.Max(1, generatedLine);
            generatedCol = Math.Max(1, generatedCol);
            sourcePath = null;
            sourceLine = Math.Max(1, generatedLine);
            sourceCol = 1;

            MoonGeneratedSourceMapFile sourceMap = LoadSourceMap(generatedFilePath);
            if (sourceMap == null)
            {
                return false;
            }

            MoonGeneratedSourceMapAnchor anchor = FindAnchorForGeneratedPosition(sourceMap, generatedLine, generatedCol);
            if (anchor?.source_span == null)
            {
                return false;
            }

            string resolvedSourcePath = ResolveSourcePath(projectRoot, generatedFilePath, sourceMap.source_file);
            if (string.IsNullOrWhiteSpace(resolvedSourcePath) || !File.Exists(resolvedSourcePath))
            {
                return false;
            }

            sourcePath = resolvedSourcePath;
            sourceLine = Math.Max(1, anchor.source_span.line);
            sourceCol = Math.Max(1, anchor.source_span.col);
            return true;
        }

        internal static MoonGeneratedSourceMapAnchor FindAnchorForGeneratedPosition(
            MoonGeneratedSourceMapFile sourceMap,
            int generatedLine,
            int generatedCol)
        {
            MoonGeneratedSourceMapAnchor anchor = FindMostSpecificAnchor(
                GetAnchors(sourceMap),
                generatedLine,
                generatedCol,
                useGeneratedNameSpan: true);

            return anchor ?? FindMostSpecificAnchor(
                GetAnchors(sourceMap),
                generatedLine,
                generatedCol,
                useGeneratedNameSpan: false);
        }

        internal static string ResolveSourcePath(string projectRoot, string generatedFilePath, string sourceFile)
        {
            if (string.IsNullOrWhiteSpace(sourceFile))
            {
                return null;
            }

            if (Path.IsPathRooted(sourceFile))
            {
                return sourceFile;
            }

            var candidates = new List<string>();
            if (!string.IsNullOrWhiteSpace(projectRoot))
            {
                candidates.Add(Path.GetFullPath(Path.Combine(projectRoot, sourceFile)));
            }

            string generatedDir = Path.GetDirectoryName(generatedFilePath) ?? string.Empty;
            candidates.Add(Path.GetFullPath(Path.Combine(generatedDir, sourceFile)));

            foreach (string candidate in candidates)
            {
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }

            return candidates.Count > 0 ? candidates[0] : null;
        }

        private static IEnumerable<MoonGeneratedSourceMapAnchor> GetAnchors(MoonGeneratedSourceMapFile sourceMap)
        {
            if (sourceMap == null)
            {
                yield break;
            }

            if (sourceMap.declaration != null)
            {
                foreach (MoonGeneratedSourceMapAnchor anchor in EnumerateAnchor(sourceMap.declaration))
                {
                    yield return anchor;
                }
            }

            foreach (MoonGeneratedSourceMapAnchor member in sourceMap.members ?? Array.Empty<MoonGeneratedSourceMapAnchor>())
            {
                if (member != null)
                {
                    foreach (MoonGeneratedSourceMapAnchor anchor in EnumerateAnchor(member))
                    {
                        yield return anchor;
                    }
                }
            }
        }

        private static IEnumerable<MoonGeneratedSourceMapAnchor> EnumerateAnchor(MoonGeneratedSourceMapAnchor anchor)
        {
            if (anchor == null)
            {
                yield break;
            }

            yield return anchor;

            foreach (MoonGeneratedSourceMapAnchor segment in anchor.segments ?? Array.Empty<MoonGeneratedSourceMapAnchor>())
            {
                foreach (MoonGeneratedSourceMapAnchor child in EnumerateAnchor(segment))
                {
                    yield return child;
                }
            }
        }

        private static MoonGeneratedSourceMapAnchor FindMostSpecificAnchor(
            IEnumerable<MoonGeneratedSourceMapAnchor> anchors,
            int line,
            int col,
            bool useGeneratedNameSpan)
        {
            MoonGeneratedSourceMapAnchor bestAnchor = null;
            int bestSize = int.MaxValue;

            foreach (MoonGeneratedSourceMapAnchor anchor in anchors)
            {
                MoonGeneratedSourceMapSpan span = useGeneratedNameSpan ? anchor.generated_name_span : anchor.generated_span;
                if (!ContainsSpan(span, line, col))
                {
                    continue;
                }

                int size = GetSpanSize(span);
                if (bestAnchor == null || size < bestSize)
                {
                    bestAnchor = anchor;
                    bestSize = size;
                }
            }

            return bestAnchor;
        }

        private static bool ContainsSpan(MoonGeneratedSourceMapSpan span, int line, int col)
        {
            if (span == null)
            {
                return false;
            }

            if (line < span.line || line > span.end_line)
            {
                return false;
            }

            if (line == span.line && col < span.col)
            {
                return false;
            }

            if (line == span.end_line && col > span.end_col)
            {
                return false;
            }

            return true;
        }

        private static int GetSpanSize(MoonGeneratedSourceMapSpan span)
        {
            if (span == null)
            {
                return int.MaxValue;
            }

            int lineDelta = Math.Max(0, span.end_line - span.line);
            int colDelta = Math.Max(0, span.end_col - span.col);
            return (lineDelta * 10000) + colDelta;
        }
    }

    internal static class MoonSourceMapJsonParser
    {
        internal static MoonGeneratedSourceMapFile Parse(string text)
        {
            var parser = new JsonParser(text);
            return ReadSourceMapFile(parser.ParseObject());
        }

        private static MoonGeneratedSourceMapFile ReadSourceMapFile(Dictionary<string, object> data)
        {
            return new MoonGeneratedSourceMapFile
            {
                version = ReadInt(data, "version"),
                source_file = ReadString(data, "source_file") ?? string.Empty,
                generated_file = ReadString(data, "generated_file") ?? string.Empty,
                declaration = ReadAnchor(data, "declaration"),
                members = ReadAnchorArray(data, "members"),
            };
        }

        private static MoonGeneratedSourceMapAnchor ReadAnchor(Dictionary<string, object> data, string key)
        {
            return ReadAnchor(ReadObject(data, key));
        }

        private static MoonGeneratedSourceMapAnchor ReadAnchor(Dictionary<string, object> data)
        {
            if (data == null)
            {
                return null;
            }

            return new MoonGeneratedSourceMapAnchor
            {
                kind = ReadString(data, "kind") ?? string.Empty,
                name = ReadString(data, "name") ?? string.Empty,
                qualified_name = ReadString(data, "qualified_name") ?? string.Empty,
                source_span = ReadSpan(data, "source_span"),
                generated_span = ReadSpan(data, "generated_span"),
                generated_name_span = ReadSpan(data, "generated_name_span"),
                segments = ReadAnchorArray(data, "segments"),
            };
        }

        private static MoonGeneratedSourceMapAnchor[] ReadAnchorArray(Dictionary<string, object> data, string key)
        {
            List<object> values = ReadArray(data, key);
            if (values == null || values.Count == 0)
            {
                return Array.Empty<MoonGeneratedSourceMapAnchor>();
            }

            var anchors = new List<MoonGeneratedSourceMapAnchor>(values.Count);
            foreach (object value in values)
            {
                MoonGeneratedSourceMapAnchor anchor = ReadAnchor(value as Dictionary<string, object>);
                if (anchor != null)
                {
                    anchors.Add(anchor);
                }
            }

            return anchors.ToArray();
        }

        private static MoonGeneratedSourceMapSpan ReadSpan(Dictionary<string, object> data, string key)
        {
            Dictionary<string, object> span = ReadObject(data, key);
            if (span == null)
            {
                return null;
            }

            return new MoonGeneratedSourceMapSpan
            {
                line = ReadInt(span, "line"),
                col = ReadInt(span, "col"),
                end_line = ReadInt(span, "end_line"),
                end_col = ReadInt(span, "end_col"),
            };
        }

        private static Dictionary<string, object> ReadObject(Dictionary<string, object> data, string key)
        {
            if (data == null || !data.TryGetValue(key, out object value))
            {
                return null;
            }

            return value as Dictionary<string, object>;
        }

        private static List<object> ReadArray(Dictionary<string, object> data, string key)
        {
            if (data == null || !data.TryGetValue(key, out object value))
            {
                return null;
            }

            return value as List<object>;
        }

        private static string ReadString(Dictionary<string, object> data, string key)
        {
            if (data == null || !data.TryGetValue(key, out object value))
            {
                return null;
            }

            return value as string;
        }

        private static int ReadInt(Dictionary<string, object> data, string key)
        {
            if (data == null || !data.TryGetValue(key, out object value) || value == null)
            {
                return 0;
            }

            switch (value)
            {
                case int intValue:
                    return intValue;
                case long longValue:
                    return (int)longValue;
                case double doubleValue:
                    return (int)doubleValue;
                case float floatValue:
                    return (int)floatValue;
                case string stringValue when int.TryParse(stringValue, NumberStyles.Integer, CultureInfo.InvariantCulture, out int parsedValue):
                    return parsedValue;
                default:
                    return Convert.ToInt32(value, CultureInfo.InvariantCulture);
            }
        }

        private sealed class JsonParser
        {
            private readonly string _text;
            private int _index;

            internal JsonParser(string text)
            {
                _text = text ?? string.Empty;
            }

            internal Dictionary<string, object> ParseObject()
            {
                object value = ParseValue();
                SkipWhitespace();
                if (_index != _text.Length)
                {
                    throw new FormatException("Unexpected trailing content in source map JSON.");
                }

                if (value is Dictionary<string, object> data)
                {
                    return data;
                }

                throw new FormatException("Source map JSON must start with an object.");
            }

            private object ParseValue()
            {
                SkipWhitespace();
                if (_index >= _text.Length)
                {
                    throw new FormatException("Unexpected end of source map JSON.");
                }

                switch (_text[_index])
                {
                    case '{':
                        return ParseObjectCore();
                    case '[':
                        return ParseArray();
                    case '"':
                        return ParseString();
                    case 't':
                        ConsumeLiteral("true");
                        return true;
                    case 'f':
                        ConsumeLiteral("false");
                        return false;
                    case 'n':
                        ConsumeLiteral("null");
                        return null;
                    default:
                        if (_text[_index] == '-' || char.IsDigit(_text[_index]))
                        {
                            return ParseNumber();
                        }

                        throw new FormatException($"Unexpected character '{_text[_index]}' in source map JSON.");
                }
            }

            private Dictionary<string, object> ParseObjectCore()
            {
                Consume('{');
                var data = new Dictionary<string, object>(StringComparer.Ordinal);

                SkipWhitespace();
                if (TryConsume('}'))
                {
                    return data;
                }

                while (true)
                {
                    string key = ParseString();
                    SkipWhitespace();
                    Consume(':');
                    data[key] = ParseValue();
                    SkipWhitespace();

                    if (TryConsume('}'))
                    {
                        return data;
                    }

                    Consume(',');
                }
            }

            private List<object> ParseArray()
            {
                Consume('[');
                var values = new List<object>();

                SkipWhitespace();
                if (TryConsume(']'))
                {
                    return values;
                }

                while (true)
                {
                    values.Add(ParseValue());
                    SkipWhitespace();

                    if (TryConsume(']'))
                    {
                        return values;
                    }

                    Consume(',');
                }
            }

            private string ParseString()
            {
                Consume('"');
                var builder = new StringBuilder();

                while (_index < _text.Length)
                {
                    char ch = _text[_index++];
                    if (ch == '"')
                    {
                        return builder.ToString();
                    }

                    if (ch != '\\')
                    {
                        builder.Append(ch);
                        continue;
                    }

                    if (_index >= _text.Length)
                    {
                        throw new FormatException("Unexpected end of source map JSON string escape.");
                    }

                    char escaped = _text[_index++];
                    switch (escaped)
                    {
                        case '"':
                        case '\\':
                        case '/':
                            builder.Append(escaped);
                            break;
                        case 'b':
                            builder.Append('\b');
                            break;
                        case 'f':
                            builder.Append('\f');
                            break;
                        case 'n':
                            builder.Append('\n');
                            break;
                        case 'r':
                            builder.Append('\r');
                            break;
                        case 't':
                            builder.Append('\t');
                            break;
                        case 'u':
                            builder.Append(ParseUnicodeEscape());
                            break;
                        default:
                            throw new FormatException($"Unsupported escape sequence '\\{escaped}' in source map JSON.");
                    }
                }

                throw new FormatException("Unterminated string in source map JSON.");
            }

            private object ParseNumber()
            {
                int start = _index;
                if (_text[_index] == '-')
                {
                    _index++;
                }

                while (_index < _text.Length && char.IsDigit(_text[_index]))
                {
                    _index++;
                }

                bool isFloatingPoint = false;
                if (_index < _text.Length && _text[_index] == '.')
                {
                    isFloatingPoint = true;
                    _index++;
                    while (_index < _text.Length && char.IsDigit(_text[_index]))
                    {
                        _index++;
                    }
                }

                if (_index < _text.Length && (_text[_index] == 'e' || _text[_index] == 'E'))
                {
                    isFloatingPoint = true;
                    _index++;
                    if (_index < _text.Length && (_text[_index] == '+' || _text[_index] == '-'))
                    {
                        _index++;
                    }

                    while (_index < _text.Length && char.IsDigit(_text[_index]))
                    {
                        _index++;
                    }
                }

                string numberText = _text.Substring(start, _index - start);
                if (!isFloatingPoint && long.TryParse(numberText, NumberStyles.Integer, CultureInfo.InvariantCulture, out long longValue))
                {
                    return longValue;
                }

                if (double.TryParse(numberText, NumberStyles.Float, CultureInfo.InvariantCulture, out double doubleValue))
                {
                    return doubleValue;
                }

                throw new FormatException($"Invalid number '{numberText}' in source map JSON.");
            }

            private char ParseUnicodeEscape()
            {
                if (_index + 4 > _text.Length)
                {
                    throw new FormatException("Incomplete unicode escape in source map JSON.");
                }

                string hex = _text.Substring(_index, 4);
                _index += 4;
                return (char)Convert.ToInt32(hex, 16);
            }

            private void ConsumeLiteral(string literal)
            {
                if (_index + literal.Length > _text.Length || string.CompareOrdinal(_text, _index, literal, 0, literal.Length) != 0)
                {
                    throw new FormatException($"Expected '{literal}' in source map JSON.");
                }

                _index += literal.Length;
            }

            private void Consume(char expected)
            {
                SkipWhitespace();
                if (_index >= _text.Length || _text[_index] != expected)
                {
                    throw new FormatException($"Expected '{expected}' in source map JSON.");
                }

                _index++;
            }

            private bool TryConsume(char expected)
            {
                SkipWhitespace();
                if (_index < _text.Length && _text[_index] == expected)
                {
                    _index++;
                    return true;
                }

                return false;
            }

            private void SkipWhitespace()
            {
                while (_index < _text.Length && char.IsWhiteSpace(_text[_index]))
                {
                    _index++;
                }
            }
        }
    }
}