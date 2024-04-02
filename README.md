# Hops to Hitler

classic [Wikipedia Game](https://en.wikipedia.org/wiki/Wikipedia:Wiki_Game) counting the number of pages to reach Hitler.

## Usage

Just call the application passing the URL of a Wikipedia page. The full URL is needed to use the actual language. E.g.
```bash
hops_to_hitler http://en.wikipedia.org/wiki/Minecraft
```

### Output

Output will contains the full chain and the search duration. E.g.
```bash
Found Hitler in 4 hop:
[
    "/wiki/Minecraft",
    "/wiki/Microsoft_Studios",
    "/wiki/Bill_Gates",
    "/wiki/Adolf_Hitler",
]
duration: 14s
```
