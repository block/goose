package gosling.example

import io.gosling.Client

fun main() {
    val client = Client()
    val pong = client.ping("aaif.io")
    println(pong.message)
}
