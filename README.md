# Twitch API Redeem Tool

*Only lightly tested so far*

This is a command line program to allow toggling the redeem set for a given twitch channel.

## Examples

In the following examples we'll use the following placeholders for command line arguments. When actually running commands, replace these placeholders with your actual values.

* `TWICTH_LOGIN`: Your twitch username
* `APP_ID`: The ID for your twitch dev console app.
* `APP_SECRET`: The secret for your twitch dev console app.
* `ADDRESS`: A localhost address to use for oauth. Must match the one in your twitch dev console app.
* `ACCESS_TOKEN`: A token to allow access to the API provided by Twitch. Can be used in place of the app_id, app_secret, and address. Likely aquired by running a command using the app_id, app_secret, and address before.
* `myfile.lua`: A path to lua file to be evaluated as part of running the command, to determine what api calls to make. See the lua examples in the sections for the relevant commands.

We'll show examples of alternately using both the app_id, app_secret, and address as well as the access token, for each command.

### Get current rewards values

See what the IDs for channel rewards are, as well as the current state of them.

```
tart TWICTH_LOGIN --app_id APP_ID --app_secret APP_SECRET --address ADDRESS get_rewards
```

```
tart TWICTH_LOGIN --token ACCESS_TOKEN get_rewards
```

### Modify rewards values

```
tart TWICTH_LOGIN --app_id APP_ID --app_secret APP_SECRET --address ADDRESS modify_rewards --lua myfile.lua
```

```
tart TWICTH_LOGIN --token ACCESS_TOKEN modify_rewards --lua myfile.lua
```

#### Lua examples

All fields filled with example values:
```
{
    broadcaster_id = "000000000",
    reward_id = "00000000-0000-0000-0000-000000000000",
    body = {
        title = "a title",
        prompt = "a prompt",
        cost = 42,
        background_color = "#888888",
        is_enabled = true,
        is_user_input_required = false,
        is_max_per_stream_enabled = false,
        max_per_stream = 24,
        is_max_per_user_per_stream_enabled = true,
        max_per_user_per_stream = 2,
        is_global_cooldown_enabled = true,
        global_cooldown_seconds = 300,
        is_paused = false,
        should_redemptions_skip_request_queue = false,
    },
}
```
Note that the `broadcaster_id` is optional and if left out will default to the id of the login passed on the command line.


Changing just one value with table literal
```
{
    reward_id = "00000000-0000-0000-0000-000000000000",
    body = {
        is_paused = true,
    },
}
```

Changing multiple rewards with array literal
```
{
    {
        reward_id = "00000000-0000-0000-0000-000000000000",
        body = {
            is_paused = true,
        },
    },
    {
        reward_id = "00000000-0000-0000-0000-111111111111",
        body = {
            is_paused = false,
        },
    },
}
```

Changing multiple rewards with IIFE, so we can use imperative statements while still having the code evaluate to a single expression
```
(function ()
    require "table"

    output = {}

    reward_ids = {
        "00000000-0000-0000-0000-000000000000",
        "00000000-0000-0000-0000-111111111111",
    }

    for _, reward_id in ipairs(reward_ids) do
      table.insert(output, {
        reward_id = reward_id,
        body = {
            is_paused = true,
        },
      })
    end

    return output
end)()
```
