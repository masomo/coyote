<?php

require "php/Relay.php";

$relay = new Coyote\Relay($argv[1]);

while ($body = $relay->next()) {
    $req = json_decode($body, true);
    $relay->send(json_encode(["hello" => $req["name"]]));
}
