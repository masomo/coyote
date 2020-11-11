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
    private $fp;

    public function __construct(string $sock, int $connectTimeout = 10)
    {
        $fp = stream_socket_client("unix://".$sock, $errno, $errstr, $connectTimeout);
        if (!$fp) {
            throw new \Exception(sprintf("could not connect to %s: %s (%d)", $sock, $errstr, $errno));
        }
        $this->fp = $fp;
        $this->sendIdentity();
    }

    public function next(): ?string
    {
        try {
            $type = $this->readType();
            if ($type !== self::MESSAGE_TYPE_REQUEST) {
                throw new \Exception("expected Request message, got: %d", $type);
            }
            $size = $this->readSize();
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

    private function readType(): int
    {
        $data = $this->read(self::TYPE_LENGTH);
        $type = unpack("Ctype", $data);
        if (false === $type) {
            throw new \Exception(sprintf("could not unpack type: %s", $data));
        }
        return $type["type"];
    }

    private function readSize(): int
    {
        $data = $this->read(self::SIZE_LENGTH);
        $size = unpack("Jsize", $data);
        if (false === $size) {
            throw new \Exception(sprintf("could not unpack size: %s", $data));
        }
        return $size["size"];
    }

    private function sendIdentity()
    {
        $this->write(self::MESSAGE_TYPE_IDENTITY, pack("N", getmypid()));
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
                fwrite($this->fp, pack("C", $type));
                fwrite($this->fp, $payload);
                break;

            case self::MESSAGE_TYPE_RESPONSE:
                fwrite($this->fp, pack("C", $type));
                fwrite($this->fp, pack("J", mb_strlen($payload, "8bit")));
                fwrite($this->fp, $payload);
                break;
            
            default:
                throw new \Exception("unknown message type: %d", $type);
                break;
        }
    }
}
