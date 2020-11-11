<?php

require "php/Relay.php";

$relay = new Coyote\Relay($argv[1]);

while ($body = $relay->next()) {
    $relay->send($body);
}
