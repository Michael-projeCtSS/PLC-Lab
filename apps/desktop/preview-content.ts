// Development-server projection of public repository content. No PLC execution,
// project state, hidden exercise tests, repairs, hints or learner progress are served.
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import type {
  ArticleDto,
  CatalogEntryDto,
  CatalogDto,
  LessonDto,
  LibraryDto,
  SourceDto,
} from "@plc-lab/ipc";

type Document = Record<string, unknown>;
function object(value: unknown): Document {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("Invalid preview content object.");
  return value as Document;
}
function text(doc: Document, key: string, fallback = ""): string {
  const value = doc[key] ?? fallback;
  if (typeof value !== "string")
    throw new Error(`Invalid preview text: ${key}`);
  return value;
}
function number(doc: Document, key: string): number {
  const value = doc[key] ?? 0;
  if (typeof value !== "number" || !Number.isFinite(value))
    throw new Error(`Invalid preview number: ${key}`);
  return value;
}
function list(doc: Document, key: string): unknown[] {
  const value = doc[key] ?? [];
  if (!Array.isArray(value)) throw new Error(`Invalid preview list: ${key}`);
  return value;
}
const records = (doc: Document, key: string) => list(doc, key).map(object);
function strings(doc: Document, key: string): string[] {
  return list(doc, key).map((value) => {
    if (typeof value !== "string")
      throw new Error(`Invalid preview string list: ${key}`);
    return value;
  });
}
const optional = (doc: Document, key: string) =>
  doc[key] == null ? null : text(doc, key);
async function documents(pack: string, folder: string): Promise<Document[]> {
  const files = (await readdir(path.join(pack, folder)))
    .filter((name) => name.endsWith(".json"))
    .sort();
  return Promise.all(
    files.map(async (name) =>
      object(JSON.parse(await readFile(path.join(pack, folder, name), "utf8"))),
    ),
  );
}
function category(kind: string): string {
  return ["vendor", "family", "hardware", "diagnostic"].includes(kind)
    ? "VENDOR_SPECIFIC"
    : kind === "troubleshooting"
      ? "TROUBLESHOOTING"
      : "FUNDAMENTAL";
}
function question(doc: Document) {
  return {
    question: text(doc, "question"),
    options: strings(doc, "options"),
    answer: number(doc, "answer"),
    explanation: text(doc, "explanation"),
  };
}

export async function readPreviewContent(pack: string): Promise<{
  format: "plc-lab-ui-preview-v1";
  library: LibraryDto;
  lessons: LessonDto[];
  articles: ArticleDto[];
  catalog: CatalogDto;
}> {
  const [lessons, exercises, groups, sourceFiles] = await Promise.all([
    documents(pack, "lessons"),
    documents(pack, "exercises"),
    Promise.all(
      [
        "fundamentals",
        "instructions",
        "protocols",
        "troubleshooting",
        "vendors",
      ].map((folder) => documents(pack, folder)),
    ),
    documents(pack, "sources"),
  ]);
  const allArticles = groups.flat();
  const titles = new Map(
    [...lessons, ...exercises, ...allArticles].map((doc) => [
      text(doc, "id"),
      text(doc, "title"),
    ]),
  );
  const related = (ids: string[]) =>
    ids.flatMap((id) =>
      titles.has(id) ? [{ id, title: titles.get(id)! }] : [],
    );
  const sources = new Map<string, SourceDto>(
    sourceFiles
      .flatMap((file) => records(file, "sources"))
      .map((source) => {
        const item: SourceDto = {
          id: text(source, "id"),
          title: text(source, "title"),
          vendor: optional(source, "vendor"),
          documentType: text(source, "documentType"),
          documentNumber: optional(source, "documentNumber"),
          url: text(source, "url"),
          retrieved: text(source, "retrieved"),
          appliesTo: strings(source, "appliesTo"),
          notes: text(source, "notes"),
          confidence: text(source, "confidence").toUpperCase(),
          verification: text(source, "verification").toUpperCase(),
        };
        return [item.id, item];
      }),
  );
  const articles: ArticleDto[] = allArticles.map((doc) => {
    const fidelity =
      doc.fidelity ?? (doc.simulation ? object(doc.simulation).fidelity : null);
    const instruction = doc.instruction ? object(doc.instruction) : null;
    const troubleshooting = doc.troubleshooting
      ? object(doc.troubleshooting)
      : null;
    const parameter = (value: Document) => ({
      name: text(value, "name"),
      dataType: text(value, "type"),
      description: text(value, "description"),
    });
    return {
      id: text(doc, "id"),
      kind: text(doc, "kind"),
      category: category(text(doc, "kind")),
      title: text(doc, "title"),
      summary: text(doc, "summary"),
      vendor: optional(doc, "vendor"),
      family: optional(doc, "family"),
      protocol: optional(doc, "protocol"),
      tier: optional(doc, "tier"),
      tags: strings(doc, "tags"),
      sections: records(doc, "sections").map((section) => ({
        heading: text(section, "heading"),
        body: text(section, "body"),
      })),
      facts: records(doc, "facts").map((fact) => ({
        label: text(fact, "label"),
        value: text(fact, "value"),
        sources: strings(fact, "sources"),
      })),
      fidelity: fidelity
        ? {
            level: text(object(fidelity), "level"),
            modeled: strings(object(fidelity), "modeled"),
            approximated: strings(object(fidelity), "approximated"),
            omitted: strings(object(fidelity), "omitted"),
          }
        : null,
      instruction: instruction
        ? {
            category: text(instruction, "category"),
            inputs: records(instruction, "inputs").map(parameter),
            outputs: records(instruction, "outputs").map(parameter),
            state: text(instruction, "state"),
            retentivity: text(instruction, "retentivity"),
            timing: text(instruction, "timing"),
            simulationSupport: text(instruction, "simulationSupport"),
            equivalents: records(instruction, "equivalents").map((item) => ({
              platform: text(item, "platform"),
              name: text(item, "name"),
              notes: text(item, "notes"),
            })),
          }
        : null,
      troubleshooting: troubleshooting
        ? {
            faultId: text(troubleshooting, "faultId"),
            area: text(troubleshooting, "area"),
            target: text(troubleshooting, "target"),
            effect: text(troubleshooting, "effect"),
            symptoms: strings(troubleshooting, "symptoms"),
            evidence: strings(troubleshooting, "evidence"),
            diagnosticMethod: strings(troubleshooting, "diagnosticMethod"),
            repair: strings(troubleshooting, "repair"),
            explanation: text(troubleshooting, "explanation"),
            simulatedIn: related(strings(troubleshooting, "simulatedIn")),
          }
        : null,
      simulationProfile: doc.simulation
        ? text(object(doc.simulation), "id")
        : null,
      sources: strings(doc, "sources").flatMap((id) =>
        sources.has(id) ? [sources.get(id)!] : [],
      ),
      related: related(strings(doc, "related")),
    };
  });
  const lessonViews: LessonDto[] = lessons
    .map((lesson) => ({
      id: text(lesson, "id"),
      number: number(lesson, "number"),
      title: text(lesson, "title"),
      summary: text(lesson, "summary"),
      level: text(lesson, "level"),
      minutes: number(lesson, "minutes"),
      objectives: strings(lesson, "objectives"),
      sections: records(lesson, "sections").map((section) => ({
        sectionType: text(section, "type"),
        heading: text(section, "heading"),
        body: text(section, "body"),
        code: optional(section, "code"),
        scene: optional(section, "scene"),
        exercise: optional(section, "exercise"),
        diagram: optional(section, "diagram"),
        question: section.question ? question(object(section.question)) : null,
      })),
      checkpoint: records(lesson, "checkpoint").map(question),
      knowledgeRefs: related(strings(lesson, "knowledgeRefs")),
    }))
    .sort((a, b) => a.number - b.number);
  const entry = (
    article: ArticleDto,
    children: CatalogEntryDto[] = [],
  ): CatalogEntryDto => ({
    id: article.id,
    title: article.title,
    summary: article.summary,
    tier: article.tier,
    profile: article.simulationProfile,
    area: article.troubleshooting?.area ?? null,
    children,
  });
  const simple = (kind: string) =>
    articles
      .filter((article) => article.kind === kind)
      .sort((a, b) => a.title.localeCompare(b.title))
      .map((article) => entry(article));
  const catalog: CatalogDto = {
    fundamentals: simple("topic"),
    instructions: simple("instruction"),
    protocols: simple("protocol"),
    troubleshooting: simple("troubleshooting"),
    vendors: articles
      .filter((article) => article.kind === "vendor")
      .map((vendor) =>
        entry(
          vendor,
          articles
            .filter(
              (article) =>
                article.kind === "family" && article.vendor === vendor.id,
            )
            .map((family) =>
              entry(
                family,
                articles
                  .filter(
                    (article) =>
                      article.family === family.id &&
                      ["hardware", "diagnostic"].includes(article.kind),
                  )
                  .map((article) => entry(article)),
              ),
            ),
        ),
      ),
  };
  const exerciseViews = [...exercises]
    .sort(
      (a, b) =>
        number(a, "curriculumOrder") - number(b, "curriculumOrder") ||
        text(a, "id").localeCompare(text(b, "id")),
    )
    .map((exercise) => {
      const kind = text(exercise, "kind", "practice");
      const difficulty = text(exercise, "difficulty");
      if (
        !["practice", "commissioning", "troubleshooting"].includes(kind) ||
        !["beginner", "intermediate", "advanced"].includes(difficulty)
      )
        throw new Error("Invalid preview exercise metadata.");
      return {
        id: text(exercise, "id"),
        title: text(exercise, "title"),
        summary: text(exercise, "summary"),
        kind: kind as "practice" | "commissioning" | "troubleshooting",
        difficulty: difficulty as "beginner" | "intermediate" | "advanced",
        minutes: number(exercise, "estimatedMinutes"),
        scene: text(exercise, "scene"),
        platform: text(exercise, "platform", "generic.iec"),
        attempts: 0,
        bestScore: 0,
        maxScore: 0,
        passed: false,
      };
    });
  return {
    format: "plc-lab-ui-preview-v1",
    library: {
      // Compatibility fields are never presented as learner results in preview.
      lessons: lessonViews.map((lesson) => ({
        id: lesson.id,
        title: lesson.title,
        summary: lesson.summary,
        number: lesson.number,
        level: lesson.level,
        minutes: lesson.minutes,
        completed: false,
        checkpointCorrect: 0,
        checkpointTotal: 0,
      })),
      exercises: exerciseViews,
      platforms: [],
      scenes: [],
      notices: [],
    },
    lessons: lessonViews,
    articles,
    catalog,
  };
}
