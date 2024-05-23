local socket = require("shared.socket")
local url = require("shared/socket.url")

local b='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
local function enc(data)
    return ((data:gsub('.', function(x) 
        local r,b='',x:byte()
        for i=8,1,-1 do r=r..(b%2^i-b%2^(i-1)>0 and '1' or '0') end
        return r;
    end)..'0000'):gsub('%d%d%d?%d?%d?%d?', function(x)
        if (#x < 6) then return '' end
        local c=0
        for i=1,6 do c=c+(x:sub(i,i)=='1' and 2^(6-i) or 0) end
        return b:sub(c+1,c+1)
    end)..({ '', '==', '=' })[#data%3+1])
end

local function create_key()
    local random = math.randomseed(os.time())
    local key = ""
    for i = 1, 16 do
        key = key .. string.char(math.random(0, 255))
    end
    return enc(key)
end

local function create_handshake(host, uri, key)
    return table.concat({
        "GET " .. uri .. " HTTP/1.1",
        "Host: " .. host,
        "Connection: Upgrade",
        "Upgrade: websocket",
        "Sec-WebSocket-Version: 13",
        "Sec-WebSocket-Key: " .. key,
        "\r\n"
    }, "\r\n")
end

-- Function to connect to a WebSocket server
local function connect(url_str)
    local parsed_url = url.parse(url_str)
    local host, port = parsed_url.host, tonumber(parsed_url.port) or 80
    local uri = parsed_url.path or "/"
    
    local tcp = assert(socket.tcp())
    assert(tcp:connect(host, port))
    tcp:settimeout(0) -- Set to non-blocking mode
    
    local key = create_key()
    local request = create_handshake(host, uri, key)
    tcp:send(request)

    return tcp
end

local function xor_op(a, b)
    local result = 0
    for i = 0, 7 do
        if (a % 2^i >= 2^(i-1)) ~= (b % 2^i >= 2^(i-1)) then
            result = result + 2^(i-1)
        end
    end
    return result
end

local function receive_frame(tcp)
    local function receive_exactly(n)
        local data, err, part = tcp:receive(n)
        if not data then
            if err == "timeout" and #part > 0 then
                return part
            else
                return nil, err
            end
        end
        return data
    end

    local data, err = receive_exactly(2)
    if not data then return nil, err end

    local fin = data:byte(1)
    local len = data:byte(2) % 128
    if len == 126 then
        local extlen, err = receive_exactly(2)
        if not extlen then return nil, err end
        len = extlen:byte(1) * 256 + extlen:byte(2)
    elseif len == 127 then
        -- Not supporting lengths >65535 for simplicity
        return nil, "received length too large"
    end

    local mask
    if data:byte(2) >= 128 then
        mask, err = receive_exactly(4)
        if not mask then return nil, err end
    end

    local payload, err = receive_exactly(len)
    if not payload then return nil, err end
    
    if mask then
        local unmasked = {}
        for i = 1, len do
            unmasked[i] = string.char(xor_op(payload:byte(i), mask:byte((i - 1) % 4 + 1)))
        end
        payload = table.concat(unmasked)
    end

    return payload
end

local ws = connect("ws://127.0.0.1:8000")

local controls = ac.overrideCarControls()

function script.update(dt)
    local message = receive_frame(ws)

    if not message then
        return
    end

    controls.gas = tonumber(message:sub(1, 4)) or 0
    controls.brake = tonumber(message:sub(6, 9)) or 0
end

