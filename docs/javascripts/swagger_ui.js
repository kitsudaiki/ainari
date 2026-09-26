// Renders the OpenAPI-specs of the rest-api docu with swagger-ui. A spec is linked in the
// markdown with
//
//     [OpenAPI-spec](open_api_docu_miko.json){ .swagger-ui-spec }
//
// so the path is resolved correctly for every url-layout and the link still works as
// download-link, if swagger-ui can not be loaded.

const SWAGGER_UI_BASE = "https://unpkg.com/swagger-ui-dist@5";

let swaggerUiLoaded = null;

// Loads swagger-ui only once and only on pages, which contain a spec
function loadSwaggerUi() {
  if (swaggerUiLoaded === null) {
    swaggerUiLoaded = new Promise((resolve, reject) => {
      const css = document.createElement("link");
      css.rel = "stylesheet";
      css.href = `${SWAGGER_UI_BASE}/swagger-ui.css`;
      document.head.appendChild(css);

      const script = document.createElement("script");
      script.src = `${SWAGGER_UI_BASE}/swagger-ui-bundle.js`;
      script.onload = resolve;
      script.onerror = () => {
        swaggerUiLoaded = null;
        reject(new Error("failed to load swagger-ui"));
      };
      document.head.appendChild(script);
    });
  }
  return swaggerUiLoaded;
}

function renderSwaggerUi() {
  const links = document.querySelectorAll("a.swagger-ui-spec");
  if (links.length === 0) {
    return;
  }

  loadSwaggerUi()
    .then(() => {
      links.forEach((link) => {
        const container = document.createElement("div");
        container.className = "swagger-ui-container";
        // replace the whole paragraph, if the link is the only content of it
        const parent = link.parentElement;
        const target =
          parent.tagName === "P" && parent.textContent.trim() === link.textContent.trim()
            ? parent
            : link;
        target.replaceWith(container);
        SwaggerUIBundle({ url: link.href, domNode: container });
      });
    })
    .catch((e) => console.error(e));
}

// called on every page-load, also with instant navigation
document$.subscribe(renderSwaggerUi);
