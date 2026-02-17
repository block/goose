#!/usr/bin/perl
use strict;
use warnings;
use IO::Socket::INET;
use IO::Select;

my ($host, $port) = @ARGV;
die "Usage: connect-proxy.pl <host> <port>\n" unless $host && $port;

my $proxy_port = $ENV{SANDBOX_PROXY_PORT} || die "SANDBOX_PROXY_PORT not set\n";

my $sock = IO::Socket::INET->new(
    PeerAddr => '127.0.0.1',
    PeerPort => $proxy_port,
    Proto    => 'tcp',
) or die "Cannot connect to proxy: $!\n";

print $sock "CONNECT $host:$port HTTP/1.1\r\nHost: $host:$port\r\n\r\n";

my $status = <$sock>;
die "Proxy error: $status" unless $status && $status =~ /\b200\b/;
while (my $hdr = <$sock>) {
    last if $hdr =~ /^\r?\n$/;
}

$| = 1;
binmode STDIN;
binmode STDOUT;
binmode $sock;

my $sel = IO::Select->new($sock, \*STDIN);
while (my @ready = $sel->can_read()) {
    for my $fh (@ready) {
        my $buf;
        my $n = sysread($fh, $buf, 8192);
        exit 0 unless $n;
        if ($fh == $sock) {
            syswrite(STDOUT, $buf) or exit 0;
        } else {
            syswrite($sock, $buf) or exit 0;
        }
    }
}
