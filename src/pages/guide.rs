use leptos::prelude::*;
use leptos_meta::Title;

#[component]
pub fn Guide() -> impl IntoView {
    view! {
        <Title text="Guide" />
        <article class="guide">
            <header class="guide-head">
                <h1>"How to use Delphinus"</h1>
                <p class="lede">
                    "Delphinus turns a list of Chinese or Japanese words into flashcards. It looks each word up in an open dictionary, lets you adjust the cards, and exports them for the app you study with."
                </p>
            </header>

            <nav class="guide-toc" aria-label="On this page">
                <a href="#words">"Writing your list"</a>
                <a href="#scan">"Scanning an image"</a>
                <a href="#anki">"Anki"</a>
                <a href="#quizlet">"Quizlet"</a>
                <a href="#vaia">"Vaia"</a>
                <a href="#about">"About"</a>
            </nav>

            <section id="words">
                <h2>"Writing your list"</h2>
                <p>
                    "Put one word on each line, or separate words with commas (Chinese " <span lang="zh">"，"</span> ", Japanese " <span lang="ja">"、"</span>
                    " or a plain comma), semicolons or spaces. You can mix them freely. Repeated words are only looked up once."
                </p>
                <div class="example-pair">
                    <p lang="zh">"中文，复习，图书馆，气温"</p>
                    <p lang="ja">"本棚、形、空、写真"</p>
                </div>
                <p>
                    "Chinese words match in simplified or traditional characters. Japanese words match in kanji or in kana, and hiragana also finds words usually written in katakana. Words must be in their dictionary form: "
                    <span lang="ja">"食べる"</span> ", not " <span lang="ja">"食べた"</span> "."
                </p>
                <p>
                    "A phrase or sentence that is not in the dictionary as a whole is split into the words it contains, so "
                    <span lang="zh">"我们去图书馆"</span> " gives " <span lang="zh">"我们"</span> ", " <span lang="zh">"去"</span>
                    " and " <span lang="zh">"图书馆"</span> ". The split is a simple longest-match, so check the result for Japanese sentences with grammar endings."
                </p>
                <p>
                    "When a word has several entries, the most common one is selected and the rest are listed under it, ready to tick. Edits you make to a card are kept when you change the card layout; use “Undo my edits” to go back."
                </p>
            </section>

            <section id="scan">
                <h2>"Scanning an image"</h2>
                <p>
                    "Use “Scan an image”, or paste or drop a screenshot onto the word box. The text found is added to your list, one line per line of text. Small kana written inside brackets is treated as furigana and left out."
                </p>
                <p>
                    "Crop the image to the words you need and keep it sharp; scanning is slower and less accurate on busy photos. On the public instance this runs on a small server paid for by the author, so please use it with care."
                </p>
            </section>

            <section id="anki">
                <h2>"Importing into Anki"</h2>
                <ol>
                    <li>"Select “Download for Anki”. You get a text file named " <code>"delphinus_anki.txt"</code> "."</li>
                    <li>"In Anki, choose File, then Import, and pick the file."</li>
                    <li>"Choose the deck and note type. The columns are already set to Front and Back."</li>
                    <li>"Select Import."</li>
                </ol>
            </section>

            <section id="quizlet">
                <h2>"Importing into Quizlet"</h2>
                <ol>
                    <li>"Select “Copy for Quizlet”."</li>
                    <li>"Create or open a study set on " <a href="https://quizlet.com">"Quizlet"</a> " and choose Import."</li>
                    <li>"Paste. Quizlet detects the tab between term and definition and the new line between cards."</li>
                    <li>"Select Import."</li>
                </ol>
            </section>

            <section id="vaia">
                <h2>"Importing into Vaia"</h2>
                <ol>
                    <li>"Select “Copy for Vaia”."</li>
                    <li>"Open a study set on " <a href="https://app.vaia.com/home">"Vaia"</a> ", open the menu in the top right corner and choose Import flashcards."</li>
                    <li>"Paste and confirm the import."</li>
                </ol>
                <p class="aside">"Delphinus is not affiliated with Anki, Quizlet or Vaia."</p>
            </section>

            <section id="about">
                <h2>"About"</h2>
                <p>
                    "Delphinus is free software. The code is on "
                    <a href="https://github.com/mariinkys/delphinus">"GitHub"</a>
                    ", where you can also "
                    <a href="https://github.com/mariinkys/delphinus/issues/new">"report a problem or suggest an idea"</a>
                    ". You can also write to "
                    <a href="mailto:kysdev.owjga@aleeas.com">"kysdev.owjga@aleeas.com"</a> "."
                </p>
                <p>
                    "Nothing you write is stored on the server. Your word list and layout choices are saved only in this browser."
                </p>
                <p>
                    "Japanese entries come from "
                    <a href="https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project">"JMdict"</a>
                    ", the property of the "
                    <a href="https://www.edrdg.org/">"Electronic Dictionary Research and Development Group"</a>
                    ", used in conformance with the Group's "
                    <a href="https://www.edrdg.org/edrdg/licence.html">"licence"</a>
                    ". Chinese entries come from "
                    <a href="https://cc-cedict.org/wiki/">"CC-CEDICT"</a>
                    ", licensed under Creative Commons Attribution-ShareAlike 4.0. Image scanning uses "
                    <a href="https://github.com/GreatV/oar-ocr">"oar-ocr"</a>
                    " with PaddleOCR models."
                </p>
                <p>
                    "The public instance is paid for by me, the author. If it helps you, consider "
                    <a href="https://www.buymeacoffee.com/mariinkys">"buying me a coffee"</a> "."
                </p>
            </section>
        </article>
    }
}
