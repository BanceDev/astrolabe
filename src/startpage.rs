pub fn get_startpage() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>New Tab</title>
    <style>
        :root {
            --bg-color: #121212;
            --text-color: #fff;
            --border-color: #444;
            --button-bg: #121212;
            --button-hover: #444;
        }
        @media (prefers-color-scheme: light) {
            :root {
                --bg-color: #f5f5f5;
                --text-color: #000;
                --border-color: #ccc;
                --button-bg: #f5f5f5;
                --button-hover: #357ae8;
            }
        }
        body {
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            height: 100vh;
            background-color: var(--bg-color);
            color: var(--text-color);
            font-family: Arial, sans-serif;
        }
        h1 {
            margin-bottom: 20px;
        }
        .search-box {
            width: 50%;
            display: flex;
        }
        input {
            flex: 1;
            padding: 15px;
            font-size: 16px;
            border: 1px solid var(--border-color);
            border-radius: 10px 0 0 10px;
            outline: none;
            background: var(--bg-color);
            color: var(--text-color);
        }
        button {
            padding: 10px;
            font-size: 16px;
            border: 1px solid var(--border-color);
            border-left: none;
            background-color: var(--button-bg);
            color: white;
            cursor: pointer;
            border-radius: 0 5px 5px 0;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        button:hover {
            background-color: var(--button-hover);
        }
        button svg {
            width: 16px;
            height: 16px;
            fill: white;
        }
    </style>
    <script>
        function handleSearch(event) {
            event.preventDefault();
            const input = document.getElementById('search-input');
            const query = input.value.trim();
            if (query) {
                if (/^(https?:\/\/)?([\da-z.-]+)\.([a-z.]{2,6})([\/\w .-]*)*\/?$/.test(query)) {
                    window.location.href = query.startsWith('http') ? query : 'https://' + query;
                } else {
                    window.location.href = `https://www.google.com/search?q=${encodeURIComponent(query)}`;
                }
            }
        }
    </script>
</head>
<body>
    <h1>Welcome!</h1>
    <form class="search-box" onsubmit="handleSearch(event)">
        <input type="text" id="search-input" placeholder="Search or enter address" required>
        <button type="submit">
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                <path d="M21.53 20.47l-5.66-5.66a8 8 0 10-1.06 1.06l5.66 5.66a.75.75 0 001.06-1.06zM4 10a6 6 0 1112 0A6 6 0 014 10z"/>
            </svg>
        </button>
    </form>
</body>
</html>"#.to_string()
}
