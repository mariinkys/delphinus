use leptos::prelude::*;

/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="min-h-[85vh] flex items-center justify-center px-4">
            <div class="max-w-4xl w-full text-center space-y-8">
                <div class="space-y-4">
                    <h1 class="text-5xl md:text-6xl p-3 font-bold bg-linear-to-r from-primary to-secondary bg-clip-text text-transparent">
                        "Generate Flashcards Effortlessly"
                    </h1>
                    <p class="text-xl md:text-2xl text-base-content/70 max-w-2xl mx-auto">
                        "Create flashcards for Chinese and Japanese in seconds. Export to Anki, Quizlet, and more."
                    </p>
                </div>

                <div class="pt-4">
                    <a href="/generator" class="btn btn-primary btn-lg gap-2">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                        </svg>
                        "Start Generating"
                    </a>
                </div>

                // Features grid
                <div class="grid md:grid-cols-3 gap-6 pt-12 pb-16">
                    <div class="card bg-base-200">
                        <div class="card-body items-center text-center">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 text-primary mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                            </svg>
                            <h3 class="card-title text-lg">"Lightning Fast"</h3>
                            <p class="text-base-content/70">"Generate hundreds of flashcards in moments"</p>
                        </div>
                    </div>

                    <div class="card bg-base-200">
                        <div class="card-body items-center text-center">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 text-primary mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z" />
                            </svg>
                            <h3 class="card-title text-lg">"Universal Export"</h3>
                            <p class="text-base-content/70">"Works with Anki, Quizlet, and other platforms"</p>
                        </div>
                    </div>

                    <div class="card bg-base-200">
                        <div class="card-body items-center text-center">
                            <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 text-primary mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                            <h3 class="card-title text-lg">"Language Focused"</h3>
                            <p class="text-base-content/70">"For Chinese and Japanese learners"</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
