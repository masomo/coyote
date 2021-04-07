<?php

require "php/Relay.php";

$relay = new Coyote\Relay($argv[1]);

while ($body = $relay->next()) {
    usleep(100 * 1000); // 100ms
    $relay->send((string)getmypid());
}
