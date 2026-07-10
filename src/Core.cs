using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Threading;

namespace LocalPass
{
    internal static class PasswordGenerator
    {
        internal const string Lowercase = "abcdefghijklmnopqrstuvwxyz";
        internal const string Uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        internal const string Numbers = "0123456789";
        internal const string Symbols = "!@#$%^&*_-+=?";
        private const string Ambiguous = "O0Il1|";
        private static readonly RandomNumberGenerator Rng = RandomNumberGenerator.Create();

        internal static IList<string> GenerateMany(
            int count,
            int length,
            bool lowercase,
            bool uppercase,
            bool numbers,
            bool symbols,
            bool excludeAmbiguous)
        {
            if (count < 1 || count > 50)
                throw new ArgumentOutOfRangeException("count", "Count must be between 1 and 50.");

            List<string> results = new List<string>(count);
            HashSet<string> unique = new HashSet<string>(StringComparer.Ordinal);
            int attempts = 0;

            while (results.Count < count)
            {
                string password = Generate(length, lowercase, uppercase, numbers, symbols, excludeAmbiguous);
                if (unique.Add(password))
                    results.Add(password);

                attempts++;
                if (attempts > count * 100)
                    throw new InvalidOperationException("Could not produce a unique password batch.");
            }

            return results;
        }

        internal static string Generate(
            int length,
            bool lowercase,
            bool uppercase,
            bool numbers,
            bool symbols,
            bool excludeAmbiguous)
        {
            if (length < 8 || length > 64)
                throw new ArgumentOutOfRangeException("length", "Length must be between 8 and 64.");

            List<string> groups = new List<string>(4);
            AddGroup(groups, lowercase, Lowercase, excludeAmbiguous);
            AddGroup(groups, uppercase, Uppercase, excludeAmbiguous);
            AddGroup(groups, numbers, Numbers, excludeAmbiguous);
            AddGroup(groups, symbols, Symbols, excludeAmbiguous);

            if (groups.Count == 0)
                throw new ArgumentException("Select at least one character group.");

            StringBuilder poolBuilder = new StringBuilder();
            foreach (string group in groups)
                poolBuilder.Append(group);
            string pool = poolBuilder.ToString();

            // Rejection sampling makes every valid password equally likely.
            char[] candidate = new char[length];
            try
            {
                for (int attempt = 0; attempt < 100000; attempt++)
                {
                    for (int i = 0; i < candidate.Length; i++)
                        candidate[i] = pool[NextInt(pool.Length)];

                    if (ContainsEveryGroup(candidate, groups))
                        return new string(candidate);
                }
            }
            finally
            {
                Array.Clear(candidate, 0, candidate.Length);
            }

            throw new InvalidOperationException("Could not satisfy the selected character policy.");
        }

        private static bool ContainsEveryGroup(char[] candidate, IList<string> groups)
        {
            foreach (string group in groups)
            {
                bool found = false;
                foreach (char character in candidate)
                {
                    if (group.IndexOf(character) >= 0)
                    {
                        found = true;
                        break;
                    }
                }
                if (!found)
                    return false;
            }
            return true;
        }

        private static void AddGroup(List<string> groups, bool enabled, string characters, bool excludeAmbiguous)
        {
            if (!enabled)
                return;

            if (!excludeAmbiguous)
            {
                groups.Add(characters);
                return;
            }

            StringBuilder filtered = new StringBuilder(characters.Length);
            foreach (char character in characters)
            {
                if (Ambiguous.IndexOf(character) < 0)
                    filtered.Append(character);
            }
            groups.Add(filtered.ToString());
        }

        private static int NextInt(int exclusiveMaximum)
        {
            if (exclusiveMaximum <= 0)
                throw new ArgumentOutOfRangeException("exclusiveMaximum");

            byte[] bytes = new byte[4];
            try
            {
                ulong range = 1UL << 32;
                ulong ceiling = range - (range % (uint)exclusiveMaximum);
                uint value;
                do
                {
                    Rng.GetBytes(bytes);
                    value = BitConverter.ToUInt32(bytes, 0);
                }
                while ((ulong)value >= ceiling);
                return (int)(value % (uint)exclusiveMaximum);
            }
            finally
            {
                Array.Clear(bytes, 0, bytes.Length);
            }
        }
    }

    internal sealed class SensitiveClipboard
    {
        private const uint CfUnicodeText = 13;
        private const uint GmemMoveable = 0x0002;
        private const uint GmemZeroInit = 0x0040;
        private const string ExclusionFormatName = "ExcludeClipboardContentFromMonitorProcessing";

        private readonly Func<IntPtr> _ownerHandle;
        private uint _ownedSequence;

        internal SensitiveClipboard(Func<IntPtr> ownerHandle)
        {
            _ownerHandle = ownerHandle;
        }

        internal bool TryCopy(string text)
        {
            if (String.IsNullOrEmpty(text))
                return false;

            uint exclusionFormat = RegisterClipboardFormat(ExclusionFormatName);
            if (exclusionFormat == 0)
                return false;

            IntPtr markerMemory = IntPtr.Zero;
            IntPtr textMemory = IntPtr.Zero;
            bool clipboardOpen = false;
            bool copied = false;
            uint copiedSequence = 0;

            try
            {
                markerMemory = GlobalAlloc(GmemMoveable | GmemZeroInit, new UIntPtr(4));
                textMemory = AllocateUnicodeText(text);
                if (markerMemory == IntPtr.Zero || textMemory == IntPtr.Zero)
                    return false;

                clipboardOpen = TryOpenClipboard(8);
                if (!clipboardOpen || !EmptyClipboard())
                    return false;

                // Install the history/cloud exclusion before exposing secret text.
                if (SetClipboardData(exclusionFormat, markerMemory) == IntPtr.Zero)
                    return false;
                markerMemory = IntPtr.Zero;

                if (SetClipboardData(CfUnicodeText, textMemory) == IntPtr.Zero)
                {
                    EmptyClipboard();
                    return false;
                }
                textMemory = IntPtr.Zero;

                copied = true;
            }
            finally
            {
                if (clipboardOpen)
                    CloseClipboard();
                ZeroAndFreeGlobal(markerMemory);
                ZeroAndFreeGlobal(textMemory);
            }

            if (copied)
            {
                // Windows publishes the final sequence when the clipboard closes.
                // Validate both sequence and HWND ownership so another writer can
                // never be mistaken for our value in the post-close interval.
                copiedSequence = GetClipboardSequenceNumber();
                copied = MatchesOwnedClipboard(copiedSequence, exclusionFormat);
            }
            _ownedSequence = copied ? copiedSequence : 0;
            return copied;
        }

        internal bool TryClearOwned()
        {
            return TryClearOwned(true);
        }

        internal bool TryClearOwnedOnce()
        {
            return TryClearOwned(false);
        }

        private bool TryClearOwned(bool retryOpen)
        {
            uint expectedSequence = _ownedSequence;
            if (expectedSequence == 0)
                return true;

            uint exclusionFormat = RegisterClipboardFormat(ExclusionFormatName);
            if (exclusionFormat == 0)
                return false;
            if (!MatchesOwnedClipboard(expectedSequence, exclusionFormat))
            {
                _ownedSequence = 0;
                return true;
            }

            if (!TryOpenClipboard(retryOpen ? 8 : 1))
                return false;

            bool cleared = false;
            try
            {
                if (!MatchesOwnedClipboard(expectedSequence, exclusionFormat))
                {
                    _ownedSequence = 0;
                    return true;
                }
                cleared = EmptyClipboard();
                if (cleared)
                    _ownedSequence = 0;
            }
            finally
            {
                CloseClipboard();
            }
            return cleared;
        }

        private bool MatchesOwnedClipboard(uint expectedSequence, uint exclusionFormat)
        {
            return expectedSequence != 0 &&
                exclusionFormat != 0 &&
                GetClipboardSequenceNumber() == expectedSequence &&
                GetClipboardOwner() == _ownerHandle() &&
                IsClipboardFormatAvailable(exclusionFormat);
        }

        private bool TryOpenClipboard(int attempts)
        {
            for (int attempt = 0; attempt < attempts; attempt++)
            {
                if (OpenClipboard(_ownerHandle()))
                    return true;
                if (attempt + 1 < attempts)
                    Thread.Sleep(15);
            }
            return false;
        }

        private static IntPtr AllocateUnicodeText(string text)
        {
            byte[] bytes = Encoding.Unicode.GetBytes(text + "\0");
            IntPtr memory = GlobalAlloc(GmemMoveable | GmemZeroInit, new UIntPtr((uint)bytes.Length));
            if (memory == IntPtr.Zero)
            {
                Array.Clear(bytes, 0, bytes.Length);
                return IntPtr.Zero;
            }

            IntPtr target = GlobalLock(memory);
            if (target == IntPtr.Zero)
            {
                GlobalFree(memory);
                Array.Clear(bytes, 0, bytes.Length);
                return IntPtr.Zero;
            }

            Marshal.Copy(bytes, 0, target, bytes.Length);
            GlobalUnlock(memory);
            Array.Clear(bytes, 0, bytes.Length);
            return memory;
        }

        private static void ZeroAndFreeGlobal(IntPtr memory)
        {
            if (memory == IntPtr.Zero)
                return;

            UIntPtr size = GlobalSize(memory);
            IntPtr target = GlobalLock(memory);
            if (target != IntPtr.Zero)
            {
                ulong length = size.ToUInt64();
                for (ulong i = 0; i < length && i <= Int32.MaxValue; i++)
                    Marshal.WriteByte(target, (int)i, 0);
                GlobalUnlock(memory);
            }
            GlobalFree(memory);
        }

        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint RegisterClipboardFormat(string format);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool OpenClipboard(IntPtr owner);

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool CloseClipboard();

        [DllImport("user32.dll", SetLastError = true)]
        private static extern bool EmptyClipboard();

        [DllImport("user32.dll", SetLastError = true)]
        private static extern IntPtr SetClipboardData(uint format, IntPtr memory);

        [DllImport("user32.dll")]
        private static extern uint GetClipboardSequenceNumber();

        [DllImport("user32.dll")]
        private static extern IntPtr GetClipboardOwner();

        [DllImport("user32.dll")]
        private static extern bool IsClipboardFormatAvailable(uint format);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr GlobalAlloc(uint flags, UIntPtr bytes);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr GlobalLock(IntPtr memory);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GlobalUnlock(IntPtr memory);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr GlobalFree(IntPtr memory);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern UIntPtr GlobalSize(IntPtr memory);
    }

    internal static class SelfTestProgram
    {
        [STAThread]
        private static int Main()
        {
            try
            {
                for (int mask = 1; mask < 16; mask++)
                {
                    bool lower = (mask & 1) != 0;
                    bool upper = (mask & 2) != 0;
                    bool numbers = (mask & 4) != 0;
                    bool symbols = (mask & 8) != 0;
                    int[] lengths = new int[] { 8, 64 };

                    foreach (int length in lengths)
                    {
                        for (int i = 0; i < 40; i++)
                        {
                            string password = PasswordGenerator.Generate(length, lower, upper, numbers, symbols, true);
                            Check(password.Length == length, "Wrong length");
                            Check(!ContainsAny(password, "O0Il1|"), "Ambiguous character generated");
                            Check(!lower || ContainsAny(password, "abcdefghijkmnopqrstuvwxyz"), "Missing lowercase");
                            Check(!upper || ContainsAny(password, "ABCDEFGHJKLMNPQRSTUVWXYZ"), "Missing uppercase");
                            Check(!numbers || ContainsAny(password, "23456789"), "Missing number");
                            Check(!symbols || ContainsAny(password, PasswordGenerator.Symbols), "Missing symbol");
                        }
                    }
                }

                IList<string> batch = PasswordGenerator.GenerateMany(50, 20, true, true, true, true, false);
                Check(batch.Count == 50, "Wrong batch size");
                Check(new HashSet<string>(batch, StringComparer.Ordinal).Count == 50, "Duplicate batch item");

                ExpectFailure(delegate { PasswordGenerator.Generate(7, true, true, true, true, true); });
                ExpectFailure(delegate { PasswordGenerator.Generate(65, true, true, true, true, true); });
                ExpectFailure(delegate { PasswordGenerator.Generate(20, false, false, false, false, true); });
                ExpectFailure(delegate { PasswordGenerator.GenerateMany(0, 20, true, true, true, true, true); });
                ExpectFailure(delegate { PasswordGenerator.GenerateMany(51, 20, true, true, true, true, true); });

                Console.WriteLine("LocalPass self-test passed.");
                return 0;
            }
            catch (Exception exception)
            {
                Console.Error.WriteLine("LocalPass self-test failed: " + exception.Message);
                return 1;
            }
        }

        private static bool ContainsAny(string value, string candidates)
        {
            foreach (char character in value)
            {
                if (candidates.IndexOf(character) >= 0)
                    return true;
            }
            return false;
        }

        private static void ExpectFailure(Action action)
        {
            bool failed = false;
            try
            {
                action();
            }
            catch (ArgumentException)
            {
                failed = true;
            }
            Check(failed, "Invalid input accepted");
        }

        private static void Check(bool condition, string message)
        {
            if (!condition)
                throw new InvalidOperationException(message);
        }
    }
}
