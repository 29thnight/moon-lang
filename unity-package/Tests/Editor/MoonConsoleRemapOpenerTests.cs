using NUnit.Framework;

namespace Moon.Editor.Tests
{
    public class MoonConsoleRemapOpenerTests
    {
        [Test]
        public void TryParseFirstMoonLocation_PrefersDiagnosticHeader()
        {
            const string text = "Assets/TestScript.mn(7,5): error [MoonRuntime] DivideByZeroException: Attempted to divide by zero.\n" +
                                "[Moon] Remapped runtime stack trace from generated Moon C#\n" +
                                "TestScript.Awake () (at Assets/TestScript.mn:7) [Moon col 5]";

            bool parsed = MoonConsoleRemapOpener.TryParseFirstMoonLocation(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.mn", sourcePath);
            Assert.AreEqual(7, sourceLine);
            Assert.AreEqual(5, sourceCol);
        }

        [Test]
        public void TryParseFirstMoonFrame_ExtractsPathLineAndColumn()
        {
            const string text = "[Moon] Remapped runtime stack trace from generated Moon C#\n" +
                                "DivideByZeroException: Attempted to divide by zero.\n" +
                                "TestScript.Awake () (at Assets/TestScript.mn:15) [Moon col 17]";

            bool parsed = MoonConsoleRemapOpener.TryParseFirstMoonFrame(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.mn", sourcePath);
            Assert.AreEqual(15, sourceLine);
            Assert.AreEqual(17, sourceCol);
        }

        [Test]
        public void TryParseFirstMoonLocation_ParsesDotNetMoonFrame()
        {
            const string text = "[Moon] Remapped runtime stack trace from generated Moon C#\n" +
                                "at TestScript.Awake() in Assets/TestScript.mn:line 15 [Moon col 17]";

            bool parsed = MoonConsoleRemapOpener.TryParseFirstMoonLocation(text, out string sourcePath, out int sourceLine, out int sourceCol);

            Assert.IsTrue(parsed);
            Assert.AreEqual("Assets" + System.IO.Path.DirectorySeparatorChar + "TestScript.mn", sourcePath);
            Assert.AreEqual(15, sourceLine);
            Assert.AreEqual(17, sourceCol);
        }

        [Test]
        public void TryParseFirstMoonFrame_ReturnsFalseWhenNoMoonFrameExists()
        {
            bool parsed = MoonConsoleRemapOpener.TryParseFirstMoonFrame(
                "DivideByZeroException: Attempted to divide by zero.\nTestScript.Awake () (at ./Packages/com.moon.generated/Runtime/TestScript.cs:32)",
                out _,
                out _,
                out _);

            Assert.IsFalse(parsed);
        }
    }
}