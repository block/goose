package aaif.example

import aaif.goose.Client

fun main() {
    val client = Client()
    val pong = client.ping("aaif.io")
    println(pong.message)
}
