/** Read-only presentation boundary. Execution, files, projects and grading stay native. */
import {
  api,
  validators,
  type ArticleDto,
  type CatalogDto,
  type LessonDto,
  type LibraryDto,
  type SearchHitDto,
} from "@plc-lab/ipc";

export const uiPreview = () => !api.available();
type PreviewContent = {
  library: LibraryDto;
  catalog: CatalogDto;
  lessons: LessonDto[];
  articles: ArticleDto[];
};
let pending: Promise<PreviewContent> | undefined;
async function preview(): Promise<PreviewContent> {
  if (!import.meta.env.DEV)
    throw new Error(
      "UI preview content requires the development server. Start TEST_PLC_LAB_WEB.bat.",
    );
  return (pending ??= fetch("/__plc_lab_preview__/content", {
    cache: "no-store",
  })
    .then(async (response) => {
      if (!response.ok)
        throw new Error(
          "The read-only UI preview content could not be loaded. Check the development terminal.",
        );
      const value: unknown = await response.json();
      if (
        !value ||
        typeof value !== "object" ||
        !("format" in value) ||
        value.format !== "plc-lab-ui-preview-v1" ||
        !("lessons" in value) ||
        !Array.isArray(value.lessons) ||
        !("articles" in value) ||
        !Array.isArray(value.articles) ||
        !("library" in value) ||
        !("catalog" in value)
      )
        throw new Error("Invalid UI preview content format.");
      return {
        library: validators.library(value.library, "preview.library"),
        catalog: validators.catalog(value.catalog, "preview.catalog"),
        lessons: value.lessons.map((item: unknown) =>
          validators.lesson(item, "preview.lesson"),
        ),
        articles: value.articles.map((item: unknown) =>
          validators.article(item, "preview.article"),
        ),
      };
    })
    .catch((error: unknown) => {
      pending = undefined;
      throw error;
    }));
}
export const presentation = {
  library: async () =>
    uiPreview() ? (await preview()).library : api.library(),
  catalog: async () =>
    uiPreview() ? (await preview()).catalog : api.catalog(),
  lesson: async (id: string) => {
    if (!uiPreview()) return api.lesson(id);
    const item = (await preview()).lessons.find((lesson) => lesson.id === id);
    if (!item) throw new Error(`Lesson is not available in UI preview: ${id}`);
    return item;
  },
  article: async (id: string) => {
    if (!uiPreview()) return api.article(id);
    const item = (await preview()).articles.find(
      (article) => article.id === id,
    );
    if (!item) throw new Error(`Article is not available in UI preview: ${id}`);
    return item;
  },
  search: async (
    query: string,
    filters: { category?: string | null } = {},
  ): Promise<SearchHitDto[]> => {
    if (!uiPreview()) return api.search(query, filters);
    const content = await preview();
    const words = query.toLocaleLowerCase().split(/\s+/).filter(Boolean);
    return [
      ...content.articles.map((item) => ({ ...item, docType: "article" })),
      ...content.lessons.map((item) => ({
        ...item,
        docType: "lesson",
        category: "PLC_LAB_SIMULATION",
        vendor: null,
      })),
      ...content.library.exercises.map((item) => ({
        ...item,
        docType: "exercise",
        category: "PLC_LAB_SIMULATION",
        vendor: null,
      })),
    ]
      .filter(
        (item) =>
          (!filters.category || item.category === filters.category) &&
          words.every((word) =>
            `${item.title} ${item.summary}`.toLocaleLowerCase().includes(word),
          ),
      )
      .slice(0, 40)
      .map((item) => ({
        id: item.id,
        docType: item.docType,
        category: item.category,
        vendor: item.vendor,
        title: item.title,
        summary: item.summary,
        snippet: item.summary,
      }));
  },
};
