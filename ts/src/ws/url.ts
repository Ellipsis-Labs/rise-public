export const toWebSocketUrl = (
  apiUrl: string,
  path: string = "/v1/ws"
): string => {
  const url = new URL(path, `${apiUrl.replace(/\/+$/, "")}/`);

  switch (url.protocol) {
    case "http:":
      url.protocol = "ws:";
      break;
    case "https:":
      url.protocol = "wss:";
      break;
    case "ws:":
    case "wss:":
      break;
    default:
      throw new Error(
        `Unsupported websocket URL protocol '${url.protocol}' in '${apiUrl}'`
      );
  }

  return url.toString();
};
