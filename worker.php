<?php

while ($payload = fgets(STDIN)) {
    $req = json_decode($payload, true);
    echo json_encode(["message" => "hello " . $req["name"]]) . "\n";
}