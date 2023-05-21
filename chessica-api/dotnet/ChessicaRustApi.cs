using System.Runtime.InteropServices;
using System.Text;

namespace Chessica.Rust;

public static class ChessicaRustApi
{
    [DllImport("chessica_api", EntryPoint="get_best_move")]
    internal static extern StringHandle GetBestMove(string initialFen, string uciMoves, uint maxDepth, uint ttKeyBits, ulong rngSeed);

    [DllImport("chessica_api", EntryPoint="free_string")]
    internal static extern void FreeString(IntPtr s);

    public static bool TryGetBestMove(string initialFen, IEnumerable<string> uciMoves, uint maxDepth, uint ttKeyBits, ulong rngSeed, out string? bestMove)
    {
        var uciMovesStr = string.Join(",", uciMoves);
        using var bestMoveHandle = ChessicaRustApi.GetBestMove(initialFen, uciMovesStr, maxDepth, ttKeyBits, rngSeed);
        if (bestMoveHandle.IsInvalid)
        {
            bestMove = null;
            return false;
        }
        bestMove = bestMoveHandle.AsString();
        return true;
    }
}

internal class StringHandle : SafeHandle
{
    public StringHandle() : base(IntPtr.Zero, true) {}

    public override bool IsInvalid => handle == IntPtr.Zero;

    public string AsString()
    {
        int len = 0;
        while (Marshal.ReadByte(handle, len) != 0) { ++len; }
        byte[] buffer = new byte[len];
        Marshal.Copy(handle, buffer, 0, buffer.Length);
        return Encoding.UTF8.GetString(buffer);
    }

    protected override bool ReleaseHandle()
    {
        if (!IsInvalid)
        {
            ChessicaRustApi.FreeString(handle);
        }

        return true;
    }
}
