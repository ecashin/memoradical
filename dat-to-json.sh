#! /bin/sh
# You have to edit the result to remove the trailing comma
# before the closing square bracket.

IFS="	"

kvq() {
    s='  "'
    s=${s}"$1"
    s=${s}'": "'
    s=${s}"$2"
    s=${s}'"'
    echo "$s"
}
kv() {
    s='  "'
    s=${s}"$1"
    s=${s}'": '
    s=${s}"$2"
    echo "$s"
}

echo "["
grep -v '^#' | while read rank prompt response; do
    echo "{"
    echo `kvq "prompt" "$prompt"`,
    echo `kvq "response" "$response"`,
    echo `kv "misses" "$rank"`,
    echo `kv "hits" 0`
    echo "},"
done
echo "]"
