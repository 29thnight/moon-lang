using System.Text;
using System.Text.Json;

namespace Prism.RoslynSidecar;

internal sealed class JsonRpcConnection
{
    private readonly Stream _input;
    private readonly Stream _output;
    private readonly JsonSerializerOptions _serializerOptions;

    public JsonRpcConnection(Stream input, Stream output, JsonSerializerOptions serializerOptions)
    {
        _input = input;
        _output = output;
        _serializerOptions = serializerOptions;
    }

    public async Task<JsonRpcRequest?> ReadRequestAsync(CancellationToken cancellationToken)
    {
        var payload = await ReadMessagePayloadAsync(cancellationToken);
        if (payload is null)
        {
            return null;
        }

        return JsonSerializer.Deserialize<JsonRpcRequest>(payload, _serializerOptions);
    }

    public Task WriteSuccessAsync<T>(RpcId id, T result, CancellationToken cancellationToken)
    {
        var response = new JsonRpcSuccessResponse<T>
        {
            Id = id,
            Result = result,
        };
        return WriteMessageAsync(response, cancellationToken);
    }

    public Task WriteErrorAsync(RpcId id, int code, string message, CancellationToken cancellationToken)
    {
        var response = new JsonRpcErrorResponse
        {
            Id = id,
            Error = new JsonRpcError
            {
                Code = code,
                Message = message,
            },
        };
        return WriteMessageAsync(response, cancellationToken);
    }

    private async Task WriteMessageAsync<T>(T message, CancellationToken cancellationToken)
    {
        var payload = JsonSerializer.SerializeToUtf8Bytes(message, _serializerOptions);
        var header = Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n\r\n");

        await _output.WriteAsync(header, cancellationToken);
        await _output.WriteAsync(payload, cancellationToken);
        await _output.FlushAsync(cancellationToken);
    }

    private async Task<byte[]?> ReadMessagePayloadAsync(CancellationToken cancellationToken)
    {
        var headerBytes = new List<byte>();
        var window = new Queue<byte>(4);

        while (true)
        {
            var buffer = new byte[1];
            var bytesRead = await _input.ReadAsync(buffer, cancellationToken);
            if (bytesRead == 0)
            {
                return headerBytes.Count == 0 ? null : throw new EndOfStreamException("Unexpected EOF while reading JSON-RPC headers.");
            }

            var value = buffer[0];
            headerBytes.Add(value);
            window.Enqueue(value);
            if (window.Count > 4)
            {
                window.Dequeue();
            }

            if (window.Count == 4 && window.SequenceEqual("\r\n\r\n"u8.ToArray()))
            {
                break;
            }
        }

        var headerText = Encoding.ASCII.GetString(headerBytes.ToArray());
        var contentLength = ParseContentLength(headerText);
        if (contentLength <= 0)
        {
            throw new InvalidDataException("Content-Length header is missing or invalid.");
        }

        var payload = new byte[contentLength];
        var offset = 0;
        while (offset < contentLength)
        {
            var bytesRead = await _input.ReadAsync(payload.AsMemory(offset, contentLength - offset), cancellationToken);
            if (bytesRead == 0)
            {
                throw new EndOfStreamException("Unexpected EOF while reading JSON-RPC payload.");
            }

            offset += bytesRead;
        }

        return payload;
    }

    private static int ParseContentLength(string headerText)
    {
        foreach (var rawLine in headerText.Split(["\r\n"], StringSplitOptions.RemoveEmptyEntries))
        {
            if (!rawLine.StartsWith("Content-Length:", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var value = rawLine["Content-Length:".Length..].Trim();
            if (int.TryParse(value, out var contentLength))
            {
                return contentLength;
            }
        }

        return 0;
    }
}