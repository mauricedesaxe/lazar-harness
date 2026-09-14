package findings

type Severity string

const (
	Advisory Severity = "advisory"
	Blocking Severity = "blocking"
)

type Finding struct {
	File     string   `json:"file"`
	Line     int      `json:"line"`
	Rule     string   `json:"rule"`
	Severity Severity `json:"severity"`
	Message  string   `json:"message"`
	Source   string   `json:"source"`
}

type Result struct {
	Findings []Finding `json:"findings"`
}

func (r Result) ExitCode() int {
	code := 0
	for _, f := range r.Findings {
		if f.Severity == Blocking {
			return 2
		}
		if f.Severity == Advisory {
			code = 1
		}
	}
	return code
}

func (r Result) Count(sev Severity) int {
	n := 0
	for _, f := range r.Findings {
		if f.Severity == sev {
			n++
		}
	}
	return n
}
