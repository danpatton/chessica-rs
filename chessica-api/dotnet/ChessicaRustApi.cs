using System.Runtime.InteropServices;
using System.Text;

namespace Chessica.Rust;

public class ChessicaRustApi
{
    [DllImport("chessica_api", EntryPoint = "get_best_move")]
    private static extern int GetBestMove(uint maxDepth, uint ttKeyBits, byte[] fenBuf, UIntPtr fenLen, byte[] bestMoveBuf, UIntPtr bestMoveLen);

    public static bool TryGetBestMove(uint maxDepth, uint ttKeyBits, string inputFen, out string? bestMove)
    {
        var fenBuf = Encoding.UTF8.GetBytes(inputFen);
        var bestMoveBuf = new byte[16];
        var bestMoveLen = GetBestMove(maxDepth, ttKeyBits, fenBuf, (UIntPtr)fenBuf.Length, bestMoveBuf, (UIntPtr)bestMoveBuf.Length);
        if (bestMoveLen == 0)
        {
            bestMove = null;
            return false;
        }
        bestMove = Encoding.UTF8.GetString(bestMoveBuf.AsSpan(0, bestMoveLen));
        return true;
    }
}