
SPACER="======================================"
EG="🔷"

. "$HOME/.cargo/env"

cd /x/tonel/
source .env
OLD_COMMIT=$(git rev-parse HEAD)

echo "$EG update the source"
git pull
echo $SPACER

NEW_COMMIT=$(git rev-parse HEAD)

function check_diff {
    local file_has_changed=$(git diff --name-only $OLD_COMMIT...$NEW_COMMIT --exit-code $1)
    if [ -z "$file_has_changed" ]; then
        return 1
    else
        return 0
    fi
}

function send_message {
    sleep 2
    base_url="https://api.telegram.org/bot$TELOXIDE_TOKEN/sendMessage"
    curl -s "$base_url?chat_id=$TELOXIDE_DEVELOPER&text=$1" -o /dev/null &
}

function check_status {
    systemctl status $1 --no-pager --no-legend > /dev/null
    [[ $? = 0 ]] && e="✅" || e="❌"
    send_message "$1 status: $e"
}

if check_diff "config/*.service"; then
    echo "$EG reload the services"
    cp config/*.service /etc/systemd/system/ --force
    systemctl daemon-reload
    echo $SPACER
fi

if check_diff "src/*"; then
    echo "$EG cargo build bot"
    send_message "building bot"
    cargo build -r
    [[ $? = 0 ]] && e="✅" || e="❌"
    send_message "bot build status: $e"
    echo $SPACER

    echo "🧹 removing the teloxide database"
    rm -f stderr.log stdout.log
    rm -f teloxide.db
    echo $SPACER

    echo "🔥 restart the bot"
    systemctl restart tonel
    check_status tonel
    echo $SPACER
fi

send_message "Done: 🌩"
echo "Deploy is Done! ✅"

