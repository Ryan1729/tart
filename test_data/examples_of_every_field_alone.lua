(function () 
    require "table"

    output = {}

    broadcaster_id = "000000000"
    reward_id = "00000000-0000-0000-0000-000000000000"

    function push(body)
        table.insert(output, {
            broadcaster_id = broadcaster_id,
            reward_id = reward_id,
            body = body,
        })
    end

    push({
        title = "a title"
    })

    push({
        prompt = "a prompt"
    })

    push({
        cost = 42
    })

    push({
        background_color = "#888888"
    })

    push({
        is_enabled = true
    })

    push({
        is_user_input_required = false
    })

    push({
        is_max_per_stream_enabled = false
    })

    push({
        max_per_stream = 24
    })

    push({
        is_max_per_user_per_stream_enabled = true
    })
    
    push({
        max_per_user_per_stream = 2
    })

    push({
        is_global_cooldown_enabled = true
    })

    push({
        global_cooldown_seconds = 300
    })

    push({
        is_paused = false
    })

    push({
        should_redemptions_skip_request_queue = false
    })
    
    return output
end)()
