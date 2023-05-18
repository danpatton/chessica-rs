using System.Runtime.InteropServices;
using System.Text;

namespace Chessica.Rust;

public class ChessicaRustApi
{
    [DllImport("chessica_api", EntryPoint = "get_best_move")]
    private static extern int GetBestMove(
        uint maxDepth,
        uint ttKeyBits,
        byte[] initialFenBuf,
        UIntPtr initialFenLen,
        byte[] uciMovesBuf,
        UIntPtr uciMovesLen,
        byte[] bestMoveBuf,
        UIntPtr bestMoveLen);

    public static bool TryGetBestMove(uint maxDepth, uint ttKeyBits, string inputFen, IEnumerable<string> uciMoves, out string? bestMove)
    {
        var initialFenBuf = Encoding.UTF8.GetBytes(inputFen);
        var uciMovesBuf = Encoding.UTF8.GetBytes(string.Join(",", uciMoves));
        var bestMoveBuf = new byte[16];
        var bestMoveLen = GetBestMove(
            maxDepth,
            ttKeyBits,
            initialFenBuf,
            (UIntPtr)initialFenBuf.Length,
            uciMovesBuf,
            (UIntPtr)uciMovesBuf.Length,
            bestMoveBuf,
            (UIntPtr)bestMoveBuf.Length);
        if (bestMoveLen == 0)
        {
            bestMove = null;
            return false;
        }
        bestMove = Encoding.UTF8.GetString(bestMoveBuf.AsSpan(0, bestMoveLen));
        return true;
    }
}