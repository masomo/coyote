<?php

namespace Coyote;

// TODO: add expected, readed params.
class ReadException extends \Exception
{
}

class Relay
{
    private const MESSAGE_TYPE_IDENTITY = 0;
    private const MESSAGE_TYPE_REQUEST = 1;
    private const MESSAGE_TYPE_RESPONSE = 2;

    private const TYPE_LENGTH = 1;
    private const SIZE_LENGTH = 8;
    private const HEADER_LENGTH = Relay::TYPE_LENGTH + Relay::SIZE_LENGTH;
    private $fp;

    public function __construct(string $sock, int $connectTimeout = 10)
    {
        $fp = stream_socket_client("unix://".$sock, $errno, $errstr, $connectTimeout);
        if (!$fp) {
            throw new \Exception(sprintf("could not connect to %s: %s (%d)", $sock, $errstr, $errno));
        }
        stream_set_timeout($fp, -1);
        $this->fp = $fp;
        $this->sendIdentity();
    }

    public function next(): ?string
    {
        try {
            [$type, $size] = $this->readHeader();
            if ($type !== self::MESSAGE_TYPE_REQUEST) {
                throw new \Exception("expected Request message, got: %d", $type);
            }
            $body = $this->read($size);
            return $body;
        } catch (ReadException $e) {
            // TODO: is there a better way to detect the socket is closed? like feof or something.
            return null;
        }
    }

    public function send(string $payload)
    {
        $this->write(self::MESSAGE_TYPE_RESPONSE, $payload);
    }

    public function __destruct()
    {
        fclose($this->fp);
    }

    private function readHeader(): array
    {
        $data = $this->read(self::HEADER_LENGTH);
        $header = unpack("Ctype/Jsize", $data);
        if (false === $header) {
            throw new \Exception(sprintf("could not unpack header: %s", $data));
        }
        return [$header["type"], $header["size"]];
    }

    private function sendIdentity()
    {
        $this->write(self::MESSAGE_TYPE_IDENTITY, (string)getmypid());
    }

    private function read(int $length): string
    {
        $data = fread($this->fp, $length);
        if (false === $data) {
            // TODO: get error?
            throw new ReadException("could not read from socket");
        }

        // TODO: is it required?
        $readedSize = mb_strlen($data, '8bit');
        if ($readedSize !== $length) {
            throw new ReadException(sprintf("short read: expected %d, readed %d", $length, $readedSize));
        }

        return $data;
    }

    private function write(int $type, string $payload): void
    {
        switch ($type) {
            case self::MESSAGE_TYPE_IDENTITY:
                fwrite($this->fp, pack("CJ", $type, $payload));
                break;

            case self::MESSAGE_TYPE_RESPONSE:
                fwrite($this->fp, pack("CJ", $type, mb_strlen($payload, "8bit")));
                fwrite($this->fp, $payload);
                break;
            
            default:
                throw new \Exception("unknown message type: %d", $type);
                break;
        }
    }
}
